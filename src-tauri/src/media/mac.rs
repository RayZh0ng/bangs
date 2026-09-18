use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use super::{publish, MediaCommand, MediaHub, MediaState};

const BRIDGE: &str = "resources/libbangs_media.dylib";

/// /usr/bin/perl is an Apple platform binary, so MediaRemote still answers it.
/// The script only loads the bridge and jumps into `bangs_media_run`.
const LOADER: &str = r#"
use strict;
use warnings;
use DynaLoader;
my $library = shift @ARGV or die "missing media bridge\n";
my $handle = DynaLoader::dl_load_file($library, 0x01) or die "cannot load media bridge\n";
my $symbol = DynaLoader::dl_find_symbol($handle, "bangs_media_run") or die "media bridge is incomplete\n";
DynaLoader::dl_install_xsub("Bangs::Media::run", $symbol)->();
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeMessage {
    available: bool,
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    album: String,
    #[serde(default)]
    app_name: String,
    #[serde(default)]
    source_id: String,
    #[serde(default)]
    playing: bool,
    duration: Option<f64>,
    elapsed: Option<f64>,
    #[serde(default)]
    timestamp: f64,
    #[serde(default)]
    has_artwork: bool,
    /// Only sent when the artwork changed, to keep the pipe light.
    artwork: Option<String>,
}

impl BridgeMessage {
    fn into_state(self, artwork: &mut Option<String>) -> Option<MediaState> {
        if !self.available {
            *artwork = None;
            return None;
        }
        if self.artwork.is_some() {
            *artwork = self.artwork;
        } else if !self.has_artwork {
            *artwork = None;
        }
        Some(MediaState {
            title: self.title,
            artist: self.artist,
            album: self.album,
            app_name: self.app_name,
            source_id: self.source_id,
            playing: self.playing,
            duration: self.duration.filter(|duration| *duration > 0.0),
            elapsed: self.elapsed,
            elapsed_at: self.timestamp,
            artwork: artwork.clone(),
        })
    }
}

fn bridge_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resolve(BRIDGE, BaseDirectory::Resource)
        .ok()
        .filter(|path| path.exists())
        .or_else(|| {
            let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BRIDGE);
            local.exists().then_some(local)
        })
}

pub fn start(app: AppHandle) {
    let (sender, commands) = mpsc::channel();
    app.state::<MediaHub>().set_sender(sender);

    thread::spawn(move || {
        let Some(bridge) = bridge_path(&app) else {
            eprintln!("[media] bridge library {BRIDGE} not found");
            return;
        };
        let mut backoff = Duration::from_secs(1);
        loop {
            let started = Instant::now();
            let spawned = Command::new("/usr/bin/perl")
                .arg("-e")
                .arg(LOADER)
                .arg(&bridge)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn();
            match spawned {
                Ok(child) => run_session(&app, child, &commands),
                Err(error) => eprintln!("[media] failed to start bridge: {error}"),
            }
            publish(&app, None);

            if started.elapsed() > Duration::from_secs(30) {
                backoff = Duration::from_secs(1);
            }
            thread::sleep(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    });
}

fn run_session(app: &AppHandle, mut child: Child, commands: &Receiver<MediaCommand>) {
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        return;
    };

    let reader_app = app.clone();
    let reader = thread::spawn(move || {
        let mut artwork = None;
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            match serde_json::from_str::<BridgeMessage>(&line) {
                Ok(message) => publish(&reader_app, message.into_state(&mut artwork)),
                Err(error) => eprintln!("[media] bad bridge message: {error}"),
            }
        }
    });

    while !reader.is_finished() {
        let command = match commands.recv_timeout(Duration::from_millis(500)) {
            Ok(command) => command,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let line = match command {
            MediaCommand::Toggle => "toggle".to_string(),
            MediaCommand::Next => "next".to_string(),
            MediaCommand::Previous => "previous".to_string(),
            MediaCommand::Seek { position } => format!("seek {position}"),
        };
        if writeln!(stdin, "{line}").is_err() {
            break;
        }
    }

    // Closing stdin makes the bridge exit on its own; kill is the fallback.
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    let _ = reader.join();
}

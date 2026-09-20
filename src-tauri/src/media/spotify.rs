//! Spotify does not always reach the system now-playing session, and when it
//! does it can leave out the artwork or the track length. It answers
//! AppleScript about all of it, so this asks it directly — but only while it
//! is already running, because `tell application "Spotify"` would otherwise
//! launch it.

use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use tauri::AppHandle;

use super::{apply, MediaCommand, MediaHub, MediaState, SPOTIFY};
use crate::i18n::t;
use tauri::Manager;

const POLL: Duration = Duration::from_secs(2);
/// Nothing to ask while Spotify is closed; just look again now and then.
const IDLE_POLL: Duration = Duration::from_secs(5);
/// ASCII unit separator: it cannot appear in a track name.
const SEPARATOR: char = '\u{1f}';
/// Album art is a few hundred KB; anything beyond this is not album art.
const MAX_ARTWORK: usize = 4 * 1024 * 1024;

const NOW_PLAYING: &str = r#"
tell application "Spotify"
  if player state is stopped then return "stopped"
  set theTrack to current track
  set theArt to ""
  try
    set theArt to artwork url of theTrack
  end try
  set d to (ASCII character 31)
  return ((player state as text) & d & (name of theTrack) & d & (artist of theTrack) & d & ¬
    (album of theTrack) & d & ((duration of theTrack) as text) & d & ((player position) as text) & d & theArt)
end tell
"#;

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let mut artwork: Option<(String, String)> = None;
        let mut published: Option<MediaState> = None;

        loop {
            if !crate::platform::app_is_running(SPOTIFY) {
                if published.take().is_some() {
                    publish(&app, None);
                }
                thread::sleep(IDLE_POLL);
                continue;
            }

            let next = ask(NOW_PLAYING).and_then(|line| parse(&line, &mut artwork));
            // Retain fresh samples for the merge even when the UI would have
            // extrapolated the same position. An older system sample must
            // not win just because we suppressed a Spotify update.
            if next.is_some() || published.is_some() {
                publish(&app, next.clone());
                published = next;
            }
            thread::sleep(POLL);
        }
    });
}

/// Transport commands for when the system session does not know about Spotify.
pub fn command(command: MediaCommand) -> Result<(), String> {
    // Telling a closed Spotify to play would start it; the user just quit it.
    if !crate::platform::app_is_running(SPOTIFY) {
        return Err(t("Spotify 没在运行", "Spotify is not running").into());
    }
    let script = match command {
        MediaCommand::Toggle => "tell application \"Spotify\" to playpause".to_string(),
        MediaCommand::Next => "tell application \"Spotify\" to next track".to_string(),
        MediaCommand::Previous => "tell application \"Spotify\" to previous track".to_string(),
        MediaCommand::Seek { position } => {
            format!("tell application \"Spotify\" to set player position to {position}")
        }
    };
    ask(&script)
        .map(|_| ())
        .ok_or_else(|| t("Spotify 没有响应", "Spotify did not answer").to_string())
}

fn publish(app: &AppHandle, next: Option<MediaState>) {
    *app.state::<MediaHub>().spotify.lock().unwrap() = next;
    apply(app);
}

fn ask(script: &str) -> Option<String> {
    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !output.status.success() {
        // A refused Apple Event (no Automation permission yet) lands here.
        let message = String::from_utf8_lossy(&output.stderr);
        let message = message.trim();
        if !message.is_empty() {
            eprintln!("[spotify] {message}");
        }
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse(line: &str, artwork: &mut Option<(String, String)>) -> Option<MediaState> {
    let mut fields = line.split(SEPARATOR);
    let playing = fields.next()? == "playing";
    let title = fields.next()?.to_string();
    if title.is_empty() {
        return None;
    }
    let artist = fields.next().unwrap_or_default().to_string();
    let album = fields.next().unwrap_or_default().to_string();
    // Spotify reports the length in milliseconds and the position in seconds.
    let duration = number(fields.next())
        .map(|ms| ms / 1000.0)
        .filter(|length| *length > 0.0);
    let elapsed = number(fields.next());
    let art = fields.next().unwrap_or_default();

    Some(MediaState {
        title,
        artist,
        album,
        app_name: "Spotify".into(),
        source_id: SPOTIFY.into(),
        playing,
        duration,
        elapsed,
        elapsed_at: now_ms(),
        artwork: fetch_artwork(art, artwork),
    })
}

/// AppleScript prints reals in the system's number format.
fn number(field: Option<&str>) -> Option<f64> {
    field?.trim().replace(',', ".").parse().ok()
}

/// Spotify hands out an https URL; the webview needs the bytes.
fn fetch_artwork(url: &str, cache: &mut Option<(String, String)>) -> Option<String> {
    if url.is_empty() {
        return None;
    }
    if let Some((cached, data)) = cache.as_ref() {
        if cached == url {
            return Some(data.clone());
        }
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .ok()?;
    let bytes = client.get(url).send().ok()?.bytes().ok()?;
    if bytes.is_empty() || bytes.len() > MAX_ARTWORK {
        return None;
    }
    let mime = if bytes.starts_with(&[0x89, b'P']) {
        "image/png"
    } else {
        "image/jpeg"
    };
    let data = format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    *cache = Some((url.to_string(), data.clone()));
    Some(data)
}

fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs_f64() * 1000.0)
        .unwrap_or_default()
}

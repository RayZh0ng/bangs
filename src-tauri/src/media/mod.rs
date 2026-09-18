//! "Now playing" for whatever app currently owns the system media session:
//! MediaRemote on macOS, GlobalSystemMediaTransportControls on Windows.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
mod spotify;
#[cfg(windows)]
mod win;

/// Spotify is the one player that needs a source of its own.
pub const SPOTIFY: &str = "com.spotify.client";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub app_name: String,
    /// Bundle identifier (macOS) or AppUserModelId (Windows) of the player.
    pub source_id: String,
    pub playing: bool,
    /// Track length in seconds, when the player reports one.
    pub duration: Option<f64>,
    /// Playback position in seconds as of `elapsed_at`.
    pub elapsed: Option<f64>,
    /// Unix time in milliseconds at which `elapsed` was sampled.
    pub elapsed_at: f64,
    /// Album art as a data URL.
    pub artwork: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum MediaCommand {
    Toggle,
    Next,
    Previous,
    Seek { position: f64 },
}

#[derive(Default)]
pub struct MediaHub {
    /// What the webview sees: the two sources below, merged.
    state: Mutex<Option<MediaState>>,
    /// What the OS media session reports.
    system: Mutex<Option<MediaState>>,
    /// What Spotify reports about itself; see media/spotify.rs.
    spotify: Mutex<Option<MediaState>>,
    commands: Mutex<Option<Sender<MediaCommand>>>,
}

impl MediaHub {
    pub fn current(&self) -> Option<MediaState> {
        self.state.lock().unwrap().clone()
    }

    pub fn send(&self, command: MediaCommand) -> Result<(), String> {
        self.commands
            .lock()
            .unwrap()
            .as_ref()
            .ok_or_else(|| "media service is not running".to_string())?
            .send(command)
            .map_err(|error| error.to_string())
    }

    fn set_sender(&self, sender: Sender<MediaCommand>) {
        *self.commands.lock().unwrap() = Some(sender);
    }
}

/// Stores what the OS media session reports.
fn publish(app: &AppHandle, next: Option<MediaState>) {
    *app.state::<MediaHub>().system.lock().unwrap() = next;
    apply(app);
}

/// Merges the sources and forwards the result to the webview when it changed.
fn apply(app: &AppHandle) {
    let hub = app.state::<MediaHub>();
    let next = merge(
        hub.system.lock().unwrap().clone(),
        hub.spotify.lock().unwrap().clone(),
    );
    {
        let mut current = hub.state.lock().unwrap();
        if *current == next {
            return;
        }
        *current = next.clone();
    }
    crate::lyrics::sync(app, next.as_ref());
    let _ = app.emit("media://update", next);
}

/// The OS session is the truth, except where Spotify knows better: it is the
/// only source when the session skips it, and it fills in what the session
/// leaves out about its own track.
fn merge(system: Option<MediaState>, spotify: Option<MediaState>) -> Option<MediaState> {
    let Some(spotify) = spotify else { return system };
    let Some(mut system) = system else { return Some(spotify) };
    if system.source_id != SPOTIFY {
        // Whichever one is actually playing is the one to show.
        return Some(if spotify.playing && !system.playing { spotify } else { system });
    }
    if system.artwork.is_none() {
        system.artwork = spotify.artwork;
    }
    if system.duration.is_none() {
        system.duration = spotify.duration;
    }
    if system.elapsed.is_none() {
        system.elapsed = spotify.elapsed;
        system.elapsed_at = spotify.elapsed_at;
    }
    Some(system)
}

/// Sends a transport command to whoever owns the current state.
pub fn command(app: &AppHandle, command: MediaCommand) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if spotify_owns(app) {
        return spotify::command(command);
    }
    app.state::<MediaHub>().send(command)
}

/// True when Spotify is the only source for what is showing: the OS session
/// cannot control a player it does not know about.
#[cfg(target_os = "macos")]
fn spotify_owns(app: &AppHandle) -> bool {
    let hub = app.state::<MediaHub>();
    let system_is_spotify = hub
        .system
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|state| state.source_id == SPOTIFY);
    !system_is_spotify
        && hub
            .state
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|state| state.source_id == SPOTIFY)
}

pub fn start(app: AppHandle) {
    #[cfg(target_os = "macos")]
    {
        spotify::start(app.clone());
        mac::start(app);
    }
    #[cfg(windows)]
    win::start(app);
}

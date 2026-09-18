//! "Now playing" for whatever app currently owns the system media session:
//! MediaRemote on macOS, GlobalSystemMediaTransportControls on Windows.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

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
    state: Mutex<Option<MediaState>>,
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

/// Stores the latest state and forwards it to the webview when it changed.
fn publish(app: &AppHandle, next: Option<MediaState>) {
    {
        let hub = app.state::<MediaHub>();
        let mut current = hub.state.lock().unwrap();
        if *current == next {
            return;
        }
        *current = next.clone();
    }
    crate::lyrics::sync(app, next.as_ref());
    let _ = app.emit("media://update", next);
}

pub fn start(app: AppHandle) {
    #[cfg(target_os = "macos")]
    mac::start(app);
    #[cfg(windows)]
    win::start(app);
}

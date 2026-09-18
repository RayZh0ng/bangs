use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Whether the notch window is shown at all.
    pub visible: bool,
    pub expand_on_hover: bool,
    /// On screens without a hardware notch, shrink to a thin bar when idle.
    pub idle_handle: bool,
    /// Pop the notch open when a Claude Code session stops working.
    pub notify_claude_idle: bool,
    /// `platform::display_label` of the monitor to attach to; `None` follows
    /// the primary monitor.
    pub display: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            visible: true,
            expand_on_hover: true,
            // A black pill over browser tabs is intrusive on Windows.
            idle_handle: cfg!(windows),
            notify_claude_idle: true,
            display: None,
        }
    }
}

pub struct SettingsState(Mutex<Settings>);

impl SettingsState {
    pub fn get(&self) -> Settings {
        self.0.lock().unwrap().clone()
    }
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

pub fn init(app: &AppHandle) -> SettingsState {
    let settings = settings_path(app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    SettingsState(Mutex::new(settings))
}

/// Applies `change`, persists the result and notifies the webview.
/// Returns the previous and the new settings.
pub fn update(app: &AppHandle, change: impl FnOnce(&mut Settings)) -> (Settings, Settings) {
    let state = app.state::<SettingsState>();
    let (previous, next) = {
        let mut guard = state.0.lock().unwrap();
        let previous = guard.clone();
        change(&mut guard);
        (previous, guard.clone())
    };

    if let Some(path) = settings_path(app) {
        let written = path
            .parent()
            .map(fs::create_dir_all)
            .transpose()
            .and_then(|_| fs::write(&path, serde_json::to_vec_pretty(&next).unwrap_or_default()));
        if let Err(error) = written {
            eprintln!("[settings] failed to save to {}: {error}", path.display());
        }
    }
    let _ = app.emit("settings://changed", &next);
    (previous, next)
}

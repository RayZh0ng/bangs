//! The Paste app is macOS only; these keep the command surface identical.

use serde::Serialize;
use tauri::AppHandle;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteState {
    pub available: bool,
    pub items: Vec<()>,
}

#[derive(Default)]
pub struct PasteHub;

impl PasteHub {
    pub fn current(&self) -> PasteState {
        PasteState::default()
    }
}

pub fn start(_app: AppHandle) {}

#[tauri::command]
pub fn paste_copy(_app: AppHandle, _id: i64) -> Result<(), String> {
    Err("Paste is only available on macOS".into())
}

#[tauri::command]
pub fn paste_show() -> Result<(), String> {
    Err("Paste is only available on macOS".into())
}

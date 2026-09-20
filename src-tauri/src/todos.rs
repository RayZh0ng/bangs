//! The short list of things to do, kept on the notch.
//!
//! It is the one panel that writes: the list lives in `<config>/todos.json`,
//! is loaded at start-up and saved after every change, so it survives a
//! restart without anything else having to know about it. Nothing is kept
//! after it is done — ticking a line off takes it off the list for good.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// The panel is a glance, not a backlog: the oldest lines drop off the end.
const MAX_TODOS: usize = 60;
/// One line of text. Anything longer belongs in a real task list.
const MAX_TEXT: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: String,
    pub text: String,
    /// Unix milliseconds.
    pub created_at: u64,
}

/// The list as the webview sees it, newest first.
#[derive(Default)]
pub struct TodoHub(Mutex<Vec<Todo>>);

impl TodoHub {
    pub fn current(&self) -> Vec<Todo> {
        self.0.lock().unwrap().clone()
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("todos.json"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or_default()
}

/// Reads the saved list. A file that cannot be parsed is left alone rather
/// than overwritten, so a bad edit can still be rescued by hand.
pub fn start(app: AppHandle) {
    let todos: Vec<Todo> = path(&app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    *app.state::<TodoHub>().0.lock().unwrap() = todos;
}

/// Applies a change, saves it and tells the webview.
fn edit(app: &AppHandle, change: impl FnOnce(&mut Vec<Todo>)) {
    let todos = {
        let hub = app.state::<TodoHub>();
        let mut guard = hub.0.lock().unwrap();
        let before = guard.clone();
        change(&mut guard);
        guard.truncate(MAX_TODOS);
        if *guard == before {
            return;
        }
        guard.clone()
    };

    if let Some(path) = path(app) {
        let written = path
            .parent()
            .map(fs::create_dir_all)
            .transpose()
            .and_then(|_| fs::write(&path, serde_json::to_vec_pretty(&todos).unwrap_or_default()));
        if let Err(error) = written {
            eprintln!("[todos] failed to save to {}: {error}", path.display());
        }
    }
    let _ = app.emit("bangs://todos", todos);
}

#[tauri::command]
pub fn todo_add(app: AppHandle, text: String) {
    let text: String = text.trim().replace(['\n', '\r', '\t'], " ").chars().take(MAX_TEXT).collect();
    if text.is_empty() {
        return;
    }
    let created_at = now_ms();
    edit(&app, |todos| {
        todos.insert(0, Todo { id: next_id(created_at), text, created_at });
    });
}

/// Unique for the life of the list: two items added in the same millisecond
/// still get ids of their own.
fn next_id(created_at: u64) -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    format!("{created_at:x}-{:x}", COUNT.fetch_add(1, Ordering::Relaxed))
}

/// Ticked off, or thought better of: either way the line is gone.
#[tauri::command]
pub fn todo_remove(app: AppHandle, id: String) {
    edit(&app, |todos| todos.retain(|todo| todo.id != id));
}

//! Read-only view of the Paste app's clipboard history (bundle id
//! `gxlself.paste-tool`). Its Core Data store is opened read-only; Paste
//! itself stays the only writer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use base64::Engine;
use rusqlite::{Connection, OpenFlags};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use super::{publish, ClipItem, ClipKind, ClipSource, ClipboardState};

const POLL: Duration = Duration::from_secs(3);
const PREVIEW_CHARS: usize = 180;
/// Core Data stores dates as seconds since 2001-01-01.
const CORE_DATA_EPOCH: f64 = 978_307_200.0;
const PASTE_BUNDLE_ID: &str = "gxlself.paste-tool";
const DOWNLOAD_PAGE: &str = "https://paste.gxlself.com";

fn store_path() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    [
        home.join("Library/Containers")
            .join(PASTE_BUNDLE_ID)
            .join("Data/Library/Application Support/Paste/PasteTool.sqlite"),
        home.join("Library/Application Support/Paste/PasteTool.sqlite"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn connect() -> Option<Connection> {
    let path = store_path()?;
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .or_else(|_| {
            // Without access to the -shm file, fall back to the last
            // checkpoint instead of the live WAL.
            Connection::open_with_flags(
                format!("file:{}?immutable=1", path.display()),
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )
        })
        .ok()
}

fn read_icons(connection: &Connection) -> HashMap<String, String> {
    let Ok(mut statement) =
        connection.prepare("SELECT ZBUNDLEID, ZICONDATA FROM ZAPPICONENTITY WHERE ZICONDATA IS NOT NULL")
    else {
        return HashMap::new();
    };
    let rows = statement.query_map([], |row| {
        let bundle_id: String = row.get(0)?;
        let icon: Vec<u8> = row.get(1)?;
        Ok((bundle_id, icon))
    });
    let Ok(rows) = rows else { return HashMap::new() };

    rows.flatten()
        .map(|(bundle_id, icon)| {
            let encoded = base64::engine::general_purpose::STANDARD.encode(icon);
            (bundle_id, format!("data:image/png;base64,{encoded}"))
        })
        .collect()
}

fn preview_of(text: Option<String>, kind: ClipKind) -> String {
    let text = text.unwrap_or_default();
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !collapsed.is_empty() {
        return collapsed.chars().take(PREVIEW_CHARS).collect();
    }
    match kind {
        ClipKind::Image => "图片".to_string(),
        ClipKind::Files => "文件".to_string(),
        ClipKind::Text => "空白内容".to_string(),
    }
}

fn read_items(
    connection: &Connection,
    icons: &HashMap<String, String>,
    limit: usize,
) -> rusqlite::Result<Vec<ClipItem>> {
    let mut statement = connection.prepare(
        "SELECT Z_PK, ZTYPE, ZISPINNED, ZCREATEDAT, ZAPPBUNDLEID, substr(ZPLAINTEXT, 1, 400)
         FROM ZCLIPBOARDITEMENTITY ORDER BY ZCREATEDAT DESC LIMIT ?1",
    )?;
    let rows = statement.query_map([limit as i64], |row| {
        let kind = match row.get::<_, i64>(1)? {
            1 => ClipKind::Image,
            2 => ClipKind::Files,
            _ => ClipKind::Text,
        };
        let app: Option<String> = row.get(4)?;
        Ok(ClipItem {
            id: row.get(0)?,
            kind,
            pinned: row.get::<_, i64>(2)? != 0,
            created_at: (row.get::<_, f64>(3).unwrap_or_default() + CORE_DATA_EPOCH) * 1000.0,
            icon: app.as_ref().and_then(|bundle_id| icons.get(bundle_id).cloned()),
            app,
            preview: preview_of(row.get(5)?, kind),
        })
    })?;
    rows.collect()
}

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let mut connection = None;
        let mut icons = HashMap::new();
        loop {
            if connection.is_none() {
                connection = connect();
                if let Some(connection) = &connection {
                    icons = read_icons(connection);
                }
            }

            let limit = app.state::<super::ClipboardHub>().limit();
            match connection.as_ref().map(|db| read_items(db, &icons, limit)) {
                Some(Ok(items)) => {
                    publish(&app, ClipboardState { available: true, source: ClipSource::Paste, items });
                }
                Some(Err(error)) => {
                    // Keep showing what was read last time and try again; a
                    // hiccup is not the same as Paste being missing.
                    eprintln!("[clipboard] read failed: {error}");
                    connection = None;
                }
                // Only an absent store means Paste is not installed.
                None if store_path().is_none() => publish(&app, ClipboardState::default()),
                None => {}
            }
            thread::sleep(POLL);
        }
    });
}

/// Reads the history immediately, for when the panel asks for another page.
pub fn refresh(app: &AppHandle) {
    let Some(connection) = connect() else { return };
    let icons = read_icons(&connection);
    let limit = app.state::<super::ClipboardHub>().limit();
    if let Ok(items) = read_items(&connection, &icons, limit) {
        publish(app, ClipboardState { available: true, source: ClipSource::Paste, items });
    }
}

/// Puts a history entry back on the clipboard. Images and files are left to
/// Paste itself, which owns the richer pasteboard types.
pub fn copy(app: AppHandle, id: i64) -> Result<(), String> {
    let connection = connect().ok_or("Paste 数据库不可用")?;
    let text: Option<String> = connection
        .query_row(
            "SELECT ZPLAINTEXT FROM ZCLIPBOARDITEMENTITY WHERE Z_PK = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let text = text.filter(|text| !text.is_empty()).ok_or("这条没有文本内容")?;
    app.clipboard().write_text(text).map_err(|error| error.to_string())
}

/// Paste registers this scheme for exactly this purpose (see its AppDelegate):
/// it toggles the clipboard panel without changing which app is in front, so
/// whatever the user picks still pastes where they were typing.
const PANEL_URL: &str = "pasteg://panel";

/// Asks Paste to show its own clipboard panel.
pub fn show_panel() -> Result<(), String> {
    let opened = Command::new("/usr/bin/open")
        .arg(PANEL_URL)
        .status()
        .map_err(|error| error.to_string())?;
    if opened.success() {
        return Ok(());
    }

    // Paste builds without the panel URL can only be launched, which is not
    // what was asked for — say so instead of silently doing something else.
    let launched = Command::new("/usr/bin/open")
        .args(["-b", PASTE_BUNDLE_ID])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    Err(if launched {
        "这个 Paste 版本还不支持直接弹面板，重新构建安装后即可".to_string()
    } else {
        "没找到 Paste".to_string()
    })
}

/// Paste is not installed: send the user to its download page.
pub fn open_download_page() -> Result<(), String> {
    Command::new("/usr/bin/open")
        .arg(DOWNLOAD_PAGE)
        .status()
        .map_err(|error| error.to_string())
        .and_then(|status| status.success().then_some(()).ok_or_else(|| "无法打开下载页".to_string()))
}

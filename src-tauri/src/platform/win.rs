use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, Monitor};
use windows::Win32::Foundation::POINT;
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_LBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use super::NotchMetrics;
use crate::settings::SettingsState;
use crate::MAIN_WINDOW;

static HIDDEN_FOR_FULLSCREEN: AtomicBool = AtomicBool::new(false);

/// Everything the window needs (topmost, no taskbar entry, WS_EX_NOACTIVATE
/// via `focusable: false`) is declared in tauri.conf.json.
pub fn prepare_window(app: &AppHandle) -> tauri::Result<()> {
    super::win_drop::prepare(app);
    Ok(())
}

/// Clips the window to the notch instead of using `WS_EX_TRANSPARENT`.
///
/// Toggling that style also adds `WS_EX_LAYERED`, and a layered WebView2
/// window stops being painted, so the notch was only visible while the cursor
/// happened to rest on it. A window region keeps the window unlayered: the
/// pixels outside the region are not drawn and clicks there land on whatever
/// is underneath.
pub fn set_hit_region(app: &AppHandle, width: f64, height: f64, scale: f64) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else { return };
    let Ok(handle) = window.hwnd() else { return };

    let window_width = crate::geometry::WINDOW_WIDTH * scale;
    let left = ((window_width - width * scale) / 2.0).round() as i32;
    let right = left + (width * scale).round() as i32;
    let bottom = (height * scale).round() as i32;

    unsafe {
        let region = CreateRectRgn(left, 0, right, bottom);
        // The window owns the region once SetWindowRgn succeeds.
        if SetWindowRgn(handle, Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
        }
    }
}

/// The language the system prefers, as a tag like "zh-CN" or "en-US".
pub fn system_language() -> String {
    let mut buffer = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if length <= 1 {
        return String::new();
    }
    // The count includes the terminating null.
    String::from_utf16_lossy(&buffer[..(length as usize - 1)])
}

/// WebView2 gets the mouse moves, so the stylesheet's own cursor applies.
pub fn set_cursor(_app: &AppHandle, _shape: &str) {}

pub fn set_window_visible(app: &AppHandle, visible: bool) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = if visible { window.show() } else { window.hide() };
    }
}

/// Global cursor position in physical pixels (the process is per-monitor DPI
/// aware, so these match monitor positions).
pub fn cursor_position() -> Option<(f64, f64)> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some((point.x as f64, point.y as f64))
}

pub fn mouse_button_down() -> bool {
    let pressed = |key: VIRTUAL_KEY| unsafe { GetAsyncKeyState(i32::from(key.0)) } < 0;
    pressed(VK_LBUTTON) || pressed(VK_RBUTTON)
}

/// Opens a folder in VS Code or Cursor through their CLI shims.
pub fn open_in_editor(editor: &str, path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let command = match editor {
        "code" | "cursor" => editor,
        other => return Err(format!("unknown editor {other}")),
    };
    let status = std::process::Command::new("cmd")
        .args(["/C", command, path])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| error.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{command} could not open {path}"))
}

/// Device name such as `\\.\DISPLAY1`, unique per attached display.
pub fn display_label(monitor: &Monitor) -> Option<String> {
    monitor.name().cloned()
}

pub fn notch_metrics(_monitor: &Monitor) -> NotchMetrics {
    NotchMetrics::default()
}

/// Windows answers the question outright (see `sync_fullscreen_visibility`),
/// so the notch never has to measure windows itself.
pub fn fullscreen_over(_rect: (f64, f64, f64, f64)) -> bool {
    false
}

/// A topmost window would otherwise sit on top of full-screen video, games and
/// presentations, so step aside while one is running.
pub fn sync_fullscreen_visibility(app: &AppHandle) {
    let fullscreen = unsafe { SHQueryUserNotificationState() }
        .map(|state| {
            state == QUNS_BUSY
                || state == QUNS_RUNNING_D3D_FULL_SCREEN
                || state == QUNS_PRESENTATION_MODE
        })
        .unwrap_or(false);
    if HIDDEN_FOR_FULLSCREEN.swap(fullscreen, Ordering::Relaxed) == fullscreen {
        return;
    }
    if app.state::<SettingsState>().get().visible {
        crate::apply_visibility(app, !fullscreen);
    }
}

use std::ffi::c_void;
use std::ptr;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSCursor, NSScreen};
use tauri::{AppHandle, Manager, Monitor};
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt};

use super::NotchMetrics;
use crate::MAIN_WINDOW;

/// Kept in its own module because `tauri_panel!` expands to a set of `use`
/// items that would otherwise clash with this file's imports.
mod panel {
    // The expansion calls `WebviewWindow::app_handle`.
    use tauri::Manager;

    tauri_nspanel::tauri_panel! {
        panel!(NotchPanel {
            config: {
                can_become_key_window: false,
                can_become_main_window: false,
                is_floating_panel: true,
                hides_on_deactivate: false
            }
        })
    }
}
use panel::NotchPanel;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

const COMBINED_SESSION_STATE: i32 = 0;
const LEFT_BUTTON: u32 = 0;
const RIGHT_BUTTON: u32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreate(source: *const c_void) -> *mut c_void;
    fn CGEventGetLocation(event: *const c_void) -> CGPoint;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(object: *const c_void);
}

/// Turns the Tauri window into a non-activating NSPanel above the menu bar.
pub fn prepare_window(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(MAIN_WINDOW)
        .expect("main window is declared in tauri.conf.json");
    let panel = window.to_panel::<NotchPanel>()?;
    // The menu bar lives at MainMenu (24); Status draws on top of it.
    panel.set_level(PanelLevel::Status.value());
    // Clicking the notch must not steal focus from the frontmost app.
    panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .stationary()
            .full_screen_auxiliary()
            .ignores_cycle()
            .into(),
    );
    panel.set_has_shadow(false);
    Ok(())
}

pub fn set_window_visible(app: &AppHandle, visible: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(MAIN_WINDOW) {
            if visible {
                panel.show();
            } else {
                panel.hide();
            }
        }
    });
}

/// macOS clips through the panel's transparent pixels on its own.
pub fn set_hit_region(_app: &AppHandle, _width: f64, _height: f64, _scale: f64) {}

/// Sets the mouse cursor. The panel never becomes key, so WKWebView is never
/// asked to update the cursor itself and the webview tells us what it wants
/// (see src/lib/hover.ts). Setting it sticks until the pointer moves over
/// another app's window, which sets its own.
pub fn set_cursor(app: &AppHandle, shape: &str) {
    let shape = shape.to_string();
    let _ = app.run_on_main_thread(move || {
        let cursor = match shape.as_str() {
            "pointer" => NSCursor::pointingHandCursor(),
            "grab" => NSCursor::openHandCursor(),
            _ => NSCursor::arrowCursor(),
        };
        cursor.set();
    });
}

/// Global cursor position in points, top-left origin of the main display.
pub fn cursor_position() -> Option<(f64, f64)> {
    unsafe {
        let event = CGEventCreate(ptr::null());
        if event.is_null() {
            return None;
        }
        let point = CGEventGetLocation(event);
        CFRelease(event);
        Some((point.x, point.y))
    }
}

pub fn mouse_button_down() -> bool {
    unsafe {
        CGEventSourceButtonState(COMBINED_SESSION_STATE, LEFT_BUTTON)
            || CGEventSourceButtonState(COMBINED_SESSION_STATE, RIGHT_BUTTON)
    }
}

/// The NSScreen showing `monitor`, matched by its frame.
fn matching_screen(mtm: MainThreadMarker, monitor: &Monitor) -> Option<Retained<NSScreen>> {
    let scale = monitor.scale_factor();
    let x = monitor.position().x as f64 / scale;
    let width = monitor.size().width as f64 / scale;
    let height = monitor.size().height as f64 / scale;
    NSScreen::screens(mtm).into_iter().find(|screen| {
        let frame = screen.frame();
        (frame.origin.x - x).abs() <= 1.0
            && (frame.size.width - width).abs() <= 1.0
            && (frame.size.height - height).abs() <= 1.0
    })
}

/// Opens a folder in VS Code or Cursor; fails when that editor is missing.
pub fn open_in_editor(editor: &str, path: &str) -> Result<(), String> {
    let app = match editor {
        "code" => "Visual Studio Code",
        "cursor" => "Cursor",
        other => return Err(format!("unknown editor {other}")),
    };
    let status = std::process::Command::new("/usr/bin/open")
        .args(["-a", app, path])
        .status()
        .map_err(|error| error.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{app} could not open {path}"))
}

/// Unique, human readable display name. tao reports "Monitor #<model>" on
/// macOS, which repeats for identical displays.
pub fn display_label(monitor: &Monitor) -> Option<String> {
    let screen = matching_screen(MainThreadMarker::new()?, monitor)?;
    Some(screen.localizedName().to_string())
}

/// Reads notch and menu bar sizes from the NSScreen matching `monitor`.
/// Returns defaults when called off the main thread.
pub fn notch_metrics(monitor: &Monitor) -> NotchMetrics {
    let Some(screen) = MainThreadMarker::new().and_then(|mtm| matching_screen(mtm, monitor)) else {
        return NotchMetrics::default();
    };
    let frame = screen.frame();

    let top_inset = screen.safeAreaInsets().top;
    if top_inset > 0.0 {
        let left = screen.auxiliaryTopLeftArea();
        let right = screen.auxiliaryTopRightArea();
        let notch_width = frame.size.width - left.size.width - right.size.width;
        return NotchMetrics {
            has_notch: true,
            notch_width: if (80.0..400.0).contains(&notch_width) { notch_width } else { 200.0 },
            notch_height: top_inset,
            menu_bar_height: top_inset,
        };
    }

    let visible = screen.visibleFrame();
    let menu_bar = (frame.origin.y + frame.size.height) - (visible.origin.y + visible.size.height);
    NotchMetrics {
        menu_bar_height: if (0.0..60.0).contains(&menu_bar) { menu_bar } else { 0.0 },
        ..NotchMetrics::default()
    }
}

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Monitor, PhysicalPosition,
    PhysicalSize,
};

use crate::settings::SettingsState;
use crate::{platform, MAIN_WINDOW};

/// Fixed size of the transparent host window in logical px. The visible notch
/// animates inside it; everything outside the hit rect is click-through.
/// Keep in sync with `WINDOW` in src/lib/layout.ts.
pub const WINDOW_WIDTH: f64 = 640.0;
pub const WINDOW_HEIGHT: f64 = 280.0;

const CURSOR_POLL: Duration = Duration::from_millis(33);
const DISPLAY_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub platform: String,
    pub has_notch: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub menu_bar_height: f64,
    pub display_name: String,
}

/// Where the window's top-left corner sits in the coordinate space of
/// `platform::cursor_position`, and how many of those units a logical px is.
#[derive(Debug, Clone, Copy)]
struct Placement {
    x: f64,
    y: f64,
    scale: f64,
}

#[derive(Default)]
pub struct Geometry(Mutex<Inner>);

#[derive(Default)]
struct Inner {
    placement: Option<Placement>,
    hidden: bool,
    hit_width: f64,
    hit_height: f64,
    screen: ScreenInfo,
    monitor_signature: String,
}

impl Geometry {
    pub fn screen(&self) -> ScreenInfo {
        self.0.lock().unwrap().screen.clone()
    }

    /// The interactive area: `width` x `height` logical px, centered at the
    /// top edge of the window.
    pub fn set_hit_rect(&self, width: f64, height: f64) {
        eprintln!("[dbg] hit {width}x{height}");
        let mut inner = self.0.lock().unwrap();
        inner.hit_width = width.clamp(0.0, WINDOW_WIDTH);
        inner.hit_height = height.clamp(0.0, WINDOW_HEIGHT);
    }

    pub fn set_hidden(&self, hidden: bool) {
        self.0.lock().unwrap().hidden = hidden;
    }

    /// Converts a cursor position to window-local logical px, returning it
    /// only when it falls inside the hit rect.
    fn locate(&self, (cursor_x, cursor_y): (f64, f64)) -> Option<(f64, f64)> {
        let inner = self.0.lock().unwrap();
        let placement = inner.placement.filter(|_| !inner.hidden)?;
        let x = (cursor_x - placement.x) / placement.scale;
        let y = (cursor_y - placement.y) / placement.scale;
        let left = (WINDOW_WIDTH - inner.hit_width) / 2.0;
        let inside = x >= left && x <= left + inner.hit_width && y >= -1.0 && y <= inner.hit_height;
        inside.then_some((x, y.max(0.0)))
    }
}

fn monitor_signature(monitors: &[Monitor]) -> String {
    monitors
        .iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            format!(
                "{:?}@{},{}:{}x{}*{}",
                monitor.name(),
                position.x,
                position.y,
                size.width,
                size.height,
                monitor.scale_factor()
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn target_monitor(app: &AppHandle) -> Option<Monitor> {
    let window = app.get_webview_window(MAIN_WINDOW)?;
    let monitors = window.available_monitors().unwrap_or_default();
    app.state::<SettingsState>()
        .get()
        .display
        .and_then(|label| {
            monitors
                .iter()
                .find(|monitor| platform::display_label(monitor).as_ref() == Some(&label))
                .cloned()
        })
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| monitors.into_iter().next())
}

/// Moves the window to the top center of the target monitor and refreshes the
/// screen metrics. Must run on the main thread (NSScreen access on macOS).
pub fn place_window(app: &AppHandle) {
    let (Some(window), Some(monitor)) = (app.get_webview_window(MAIN_WINDOW), target_monitor(app))
    else {
        return;
    };
    let scale = monitor.scale_factor();
    let position = monitor.position();
    let size = monitor.size();

    let placement = if cfg!(target_os = "macos") {
        // tao positions windows in logical, top-left based global coordinates
        // on macOS, the same space CGEventGetLocation reports the cursor in.
        let x = (position.x as f64 / scale + (size.width as f64 / scale - WINDOW_WIDTH) / 2.0)
            .round();
        let y = position.y as f64 / scale;
        let _ = window.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
        let _ = window.set_position(LogicalPosition::new(x, y));
        Placement { x, y, scale: 1.0 }
    } else {
        // Windows desktop coordinates are physical pixels, and monitors can
        // have different DPI, so compute everything in the target's pixels.
        let width = (WINDOW_WIDTH * scale).round();
        let x = (position.x as f64 + (size.width as f64 - width) / 2.0).round();
        let y = position.y as f64;
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
        let _ = window.set_size(PhysicalSize::new(
            width as u32,
            (WINDOW_HEIGHT * scale).round() as u32,
        ));
        Placement { x, y, scale }
    };

    let metrics = platform::notch_metrics(&monitor);
    let screen = ScreenInfo {
        platform: std::env::consts::OS.to_string(),
        has_notch: metrics.has_notch,
        notch_width: metrics.notch_width,
        notch_height: metrics.notch_height,
        menu_bar_height: metrics.menu_bar_height,
        display_name: platform::display_label(&monitor).unwrap_or_default(),
    };

    let signature = monitor_signature(&window.available_monitors().unwrap_or_default());
    let changed = {
        let geometry = app.state::<Geometry>();
        let mut inner = geometry.0.lock().unwrap();
        inner.placement = Some(placement);
        inner.monitor_signature = signature;
        let changed = inner.screen != screen;
        inner.screen = screen.clone();
        changed
    };
    if changed {
        let _ = app.emit("bangs://screen", &screen);
    }
}

/// Re-attaches the notch when displays are added, removed or rearranged.
pub fn spawn_display_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(DISPLAY_POLL);
        let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
            continue;
        };
        let signature = monitor_signature(&window.available_monitors().unwrap_or_default());
        let changed = app.state::<Geometry>().0.lock().unwrap().monitor_signature != signature;
        if changed {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                place_window(&handle);
                crate::tray::refresh(&handle);
            });
        }
        #[cfg(windows)]
        platform::sync_fullscreen_visibility(&app);
    });
}

/// Polls the global cursor to drive hover and click-through. Polling works the
/// same on both platforms, needs no accessibility permission, and keeps
/// working while the window ignores mouse events.
///
/// It also streams the pointer position while inside: WKWebView ignores mouse
/// moves in a panel that is not key, so the webview derives hover from this.
pub fn spawn_cursor_tracker(app: AppHandle) {
    thread::spawn(move || {
        let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
            return;
        };
        let _ = window.set_ignore_cursor_events(true);
        let mut inside = false;
        let mut was_down = false;
        let mut last_pointer = None;

        loop {
            thread::sleep(CURSOR_POLL);
            let Some(cursor) = platform::cursor_position() else {
                continue;
            };

            let pointer = app.state::<Geometry>().locate(cursor);
            if pointer.is_some() != inside {
                inside = pointer.is_some();
                let _ = window.set_ignore_cursor_events(!inside);
                eprintln!("[dbg] hover {inside}");
                let _ = app.emit("bangs://hover", inside);
            }
            if let Some((x, y)) = pointer.filter(|_| pointer != last_pointer) {
                let _ = app.emit("bangs://pointer", [x, y]);
            }
            last_pointer = pointer;

            let down = platform::mouse_button_down();
            if down && !was_down && !inside {
                let _ = app.emit("bangs://outside-click", ());
            }
            was_down = down;
        }
    });
}

mod dev;
mod geometry;
mod media;
mod platform;
#[cfg(target_os = "macos")]
#[path = "paste.rs"]
mod paste;
#[cfg(not(target_os = "macos"))]
#[path = "paste_other.rs"]
mod paste;
mod settings;
mod shelf;
mod tray;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use dev::{DevHub, DevState};
use geometry::{Geometry, ScreenInfo};
use media::{MediaCommand, MediaHub, MediaState};
use paste::{PasteHub, PasteState};
use settings::{Settings, SettingsState};

pub const MAIN_WINDOW: &str = "main";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    screen: ScreenInfo,
    settings: Settings,
    media: Option<MediaState>,
    dev: DevState,
    paste: PasteState,
    drag_icon: Option<String>,
}

/// Everything the webview needs for its first render. Events that fire
/// before the webview subscribes are covered by this snapshot.
#[tauri::command]
fn bootstrap(app: AppHandle) -> Bootstrap {
    Bootstrap {
        screen: app.state::<Geometry>().screen(),
        settings: app.state::<SettingsState>().get(),
        media: app.state::<MediaHub>().current(),
        dev: app.state::<DevHub>().current(),
        paste: app.state::<PasteHub>().current(),
        drag_icon: shelf::drag_icon_path(&app),
    }
}

/// Called by the webview after its first render so the window never shows
/// an empty frame.
#[tauri::command]
fn notch_ready(app: AppHandle) {
    if app.state::<SettingsState>().get().visible {
        apply_visibility(&app, true);
    }
}

#[tauri::command]
fn debug_log(message: String) {
    eprintln!("[web] {message}");
}

#[tauri::command]
fn set_hit_rect(geometry: State<Geometry>, width: f64, height: f64) {
    geometry.set_hit_rect(width, height);
}

#[tauri::command]
fn media_command(hub: State<MediaHub>, command: MediaCommand) -> Result<(), String> {
    hub.send(command)
}

pub fn apply_visibility(app: &AppHandle, visible: bool) {
    app.state::<Geometry>().set_hidden(!visible);
    platform::set_window_visible(app, visible);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            app.manage(settings::init(&handle));
            app.manage(Geometry::default());
            app.manage(MediaHub::default());
            app.manage(DevHub::default());
            app.manage(PasteHub::default());

            platform::prepare_window(&handle)?;
            geometry::place_window(&handle);
            geometry::spawn_display_watcher(handle.clone());
            geometry::spawn_cursor_tracker(handle.clone());
            media::start(handle.clone());
            dev::start(handle.clone());
            paste::start(handle.clone());
            tray::create(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            notch_ready,
            debug_log,
            set_hit_rect,
            media_command,
            dev::open_project,
            paste::paste_copy,
            paste::paste_show,
            shelf::shelf_inspect,
            shelf::open_file,
            shelf::reveal_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_autostart::ManagerExt as _;

use crate::settings::{self, SettingsState};
use crate::{geometry, platform, MAIN_WINDOW};

const TRAY_ID: &str = "bangs-tray";
const DISPLAY_PREFIX: &str = "display:";

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let settings = app.state::<SettingsState>().get();
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let monitors = app
        .get_webview_window(MAIN_WINDOW)
        .and_then(|window| window.available_monitors().ok())
        .unwrap_or_default();

    let display = Submenu::with_id(app, "display", "显示器", true)?;
    display.append(&CheckMenuItem::with_id(
        app,
        DISPLAY_PREFIX,
        "跟随主显示器",
        true,
        settings.display.is_none(),
        None::<&str>,
    )?)?;
    for monitor in &monitors {
        let Some(label) = platform::display_label(monitor) else { continue };
        display.append(&CheckMenuItem::with_id(
            app,
            format!("{DISPLAY_PREFIX}{label}"),
            &label,
            true,
            settings.display.as_ref() == Some(&label),
            None::<&str>,
        )?)?;
    }

    Menu::with_items(
        app,
        &[
            &CheckMenuItem::with_id(app, "visible", "显示刘海", true, settings.visible, None::<&str>)?,
            &CheckMenuItem::with_id(app, "hover", "悬停时展开", true, settings.expand_on_hover, None::<&str>)?,
            &CheckMenuItem::with_id(app, "idle-handle", "空闲时收成细条", true, settings.idle_handle, None::<&str>)?,
            &CheckMenuItem::with_id(app, "lyrics", "显示歌词", true, settings.lyrics_enabled, None::<&str>)?,
            &CheckMenuItem::with_id(app, "notify-claude", "Claude 忙完时提醒", true, settings.notify_claude_idle, None::<&str>)?,
            &display,
            &PredefinedMenuItem::separator(app)?,
            &CheckMenuItem::with_id(app, "autostart", "开机启动", true, autostart, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?,
        ],
    )
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Bangs")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()));

    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?)
        .icon_as_template(true);
    #[cfg(not(target_os = "macos"))]
    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone().to_owned()),
        None => builder,
    };

    builder.build(app)?;
    Ok(())
}

/// Rebuilds the menu so check marks and the display list stay current.
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "quit" => app.exit(0),
        "visible" => {
            let (_, next) = settings::update(app, |settings| settings.visible = !settings.visible);
            crate::apply_visibility(app, next.visible);
        }
        "hover" => {
            settings::update(app, |settings| settings.expand_on_hover = !settings.expand_on_hover);
        }
        "lyrics" => {
            let (_, next) = settings::update(app, |settings| {
                settings.lyrics_enabled = !settings.lyrics_enabled;
            });
            crate::lyrics::set_enabled(app, next.lyrics_enabled);
        }
        "notify-claude" => {
            settings::update(app, |settings| {
                settings.notify_claude_idle = !settings.notify_claude_idle;
            });
        }
        "idle-handle" => {
            settings::update(app, |settings| settings.idle_handle = !settings.idle_handle);
        }
        "autostart" => {
            let launcher = app.autolaunch();
            let result = if launcher.is_enabled().unwrap_or(false) {
                launcher.disable()
            } else {
                launcher.enable()
            };
            if let Err(error) = result {
                eprintln!("[tray] failed to toggle autostart: {error}");
            }
        }
        other => {
            let Some(name) = other.strip_prefix(DISPLAY_PREFIX) else { return };
            settings::update(app, |settings| {
                settings.display = (!name.is_empty()).then(|| name.to_string());
            });
            geometry::place_window(app);
        }
    }
    refresh(app);
}

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "heic", "avif", "svg"];
const DRAG_ICON: &[u8] = include_bytes!("../icons/drag-file.png");

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    path: String,
    name: String,
    extension: String,
    size: u64,
    is_dir: bool,
    is_image: bool,
}

/// Describes dropped or restored shelf paths, skipping ones that no longer
/// exist. Images are added to the asset protocol scope so the webview can show
/// thumbnails of exactly these files and nothing else.
#[tauri::command]
pub fn shelf_inspect(app: AppHandle, paths: Vec<String>) -> Vec<FileMeta> {
    let asked = paths.len();
    let files: Vec<FileMeta> = paths
        .into_iter()
        .filter_map(|path| {
            let metadata = match std::fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    eprintln!("[shelf] cannot read {path}: {error}");
                    return None;
                }
            };
            let file = Path::new(&path);
            let extension = file
                .extension()
                .map(|ext| ext.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let is_image = metadata.is_file() && IMAGE_EXTENSIONS.contains(&extension.as_str());
            if is_image {
                let _ = app.asset_protocol_scope().allow_file(&path);
            }
            Some(FileMeta {
                name: file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone()),
                path,
                extension,
                size: metadata.len(),
                is_dir: metadata.is_dir(),
                is_image,
            })
        })
        .collect();
    if files.len() != asked {
        eprintln!("[shelf] {asked} path(s) offered, {} usable", files.len());
    }
    files
}

#[tauri::command]
pub fn open_file(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reveal_file(app: AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| error.to_string())
}

/// Preview image used when dragging a non-image file out of the shelf.
pub fn drag_icon_path(app: &AppHandle) -> Option<String> {
    let path = app.path().app_cache_dir().ok()?.join("drag-file.png");
    if !path.exists() {
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(&path, DRAG_ICON).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

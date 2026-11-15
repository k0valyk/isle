//! macOS capture bridge.
//!
//! Screenshots are taken with the system `screencapture` tool (it already asks
//! for the Screen Recording permission), then edited in the same Svelte editor.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine as _;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub path: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub preview: String,
}

pub struct CaptureViewState {
    pub path: Mutex<Option<String>>,
}

impl Default for CaptureViewState {
    fn default() -> Self {
        Self {
            path: Mutex::new(None),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFile {
    pub path: String,
    pub name: String,
    pub modified: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMonitor {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

fn pictures_dir() -> Result<PathBuf, String> {
    let dirs = directories::UserDirs::new().ok_or_else(|| "No home directory".to_string())?;
    let base = dirs
        .picture_dir()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| dirs.home_dir().join("Pictures"));
    let dir = base.join("Isle");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn preview_data_url(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:image/png;base64,{encoded}"))
}

fn monitors_from(app: &AppHandle) -> Vec<CaptureMonitor> {
    let primary = app.primary_monitor().ok().flatten();
    let primary_position = primary.as_ref().map(|monitor| monitor.position());
    app.available_monitors()
        .map(|list| {
            list.into_iter()
                .enumerate()
                .map(|(index, monitor)| {
                    let position = monitor.position();
                    let size = monitor.size();
                    CaptureMonitor {
                        index,
                        name: monitor
                            .name()
                            .cloned()
                            .unwrap_or_else(|| format!("Display {}", index + 1)),
                        x: position.x,
                        y: position.y,
                        width: size.width,
                        height: size.height,
                        primary: primary_position
                            .map(|primary| primary.x == position.x && primary.y == position.y)
                            .unwrap_or(index == 0),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn capture_monitors(app: AppHandle) -> Vec<CaptureMonitor> {
    monitors_from(&app)
}

#[tauri::command]
pub async fn capture_screenshot(
    app: AppHandle,
    monitor: Option<usize>,
) -> Result<CaptureResult, String> {
    let island = app.get_webview_window("main");
    let was_visible = island
        .as_ref()
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(true);
    let keep_position = island.as_ref().and_then(crate::window_util::physical_position);
    if was_visible {
        if let Some(window) = island.as_ref() {
            let _ = window.hide();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(220));

    let result = tauri::async_runtime::spawn_blocking(move || {
        let dir = pictures_dir()?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let name = format!("Isle-{stamp}.png");
        let path = dir.join(&name);

        let mut command = std::process::Command::new("/usr/sbin/screencapture");
        command.args(["-x", "-t", "png"]);
        if let Some(index) = monitor {
            command.arg("-D").arg((index + 1).to_string());
        }
        command.arg(&path);
        let status = command
            .status()
            .map_err(|error| format!("Could not start screencapture: {error}"))?;
        if !status.success() {
            return Err("Screen capture was cancelled or denied".into());
        }

        let image = image::open(&path).map_err(|error| error.to_string())?;
        let (width, height) = (image.width(), image.height());
        Ok(CaptureResult {
            preview: preview_data_url(&path)?,
            path: path.to_string_lossy().to_string(),
            name,
            width,
            height,
        })
    })
    .await
    .map_err(|error| error.to_string())?;

    if let Some(window) = island.as_ref() {
        if was_visible {
            let _ = window.show();
            if let Some((x, y)) = keep_position {
                crate::window_util::place_at(window, x, y);
            }
            crate::window_util::mark_tool_window(window);
        }
    }
    result
}

#[tauri::command]
pub async fn capture_list() -> Result<Vec<CaptureFile>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let dir = pictures_dir()?;
        let mut files: Vec<CaptureFile> = std::fs::read_dir(&dir)
            .map_err(|error| error.to_string())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.eq_ignore_ascii_case("png"))
                    .unwrap_or(false)
            })
            .map(|entry| {
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|value| value.modified().ok())
                    .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|value| value.as_millis() as i64)
                    .unwrap_or(0);
                CaptureFile {
                    path: entry.path().to_string_lossy().to_string(),
                    name: entry.file_name().to_string_lossy().to_string(),
                    modified,
                }
            })
            .collect();
        files.sort_by_key(|file| std::cmp::Reverse(file.modified));
        files.truncate(40);
        Ok(files)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn image_preview(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || preview_data_url(Path::new(&path)))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn image_data(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = std::fs::read(Path::new(&path)).map_err(|error| error.to_string())?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err("Image is too large to edit".into());
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(format!("data:image/png;base64,{encoded}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn save_capture_data(
    data_url: String,
    name: Option<String>,
) -> Result<CaptureResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let payload = data_url
            .split_once(',')
            .map(|(_, value)| value)
            .unwrap_or(data_url.as_str());
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|error| error.to_string())?;
        let image = image::load_from_memory(&bytes)
            .map_err(|error| error.to_string())?
            .to_rgba8();
        let (width, height) = image.dimensions();

        let dir = pictures_dir()?;
        let file_name = name.unwrap_or_else(|| {
            format!("Isle-edit-{}.png", chrono::Local::now().format("%Y%m%d-%H%M%S-%3f"))
        });
        let path = dir.join(file_name);
        image.save(&path).map_err(|error| error.to_string())?;

        Ok(CaptureResult {
            preview: preview_data_url(&path)?,
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default(),
            width,
            height,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn ocr_image(_path: String) -> Result<String, String> {
    Err("Text recognition is not available on macOS yet".into())
}

#[tauri::command]
pub async fn qr_generate(text: String, size: Option<u32>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let payload = text.trim();
        if payload.is_empty() {
            return Err("QR payload is empty".into());
        }
        let code = qrcode::QrCode::with_error_correction_level(payload.as_bytes(), qrcode::EcLevel::M)
            .map_err(|error| error.to_string())?;
        let side = size.unwrap_or(240).clamp(96, 1024) as u32;
        let image = code
            .render::<image::Luma<u8>>()
            .quiet_zone(true)
            .module_dimensions(side / code.width() as u32 + 1, side / code.width() as u32 + 1)
            .build();
        let mut bytes = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .map_err(|error| error.to_string())?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(format!("data:image/png;base64,{encoded}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn qr_scan(path: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let image = image::open(Path::new(&path))
            .map_err(|error| error.to_string())?
            .to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(image);
        let grids = prepared.detect_grids();
        let mut results = Vec::new();
        for grid in grids {
            if let Ok((_, content)) = grid.decode() {
                results.push(content);
            }
        }
        if results.is_empty() {
            return Err("No QR code found".into());
        }
        Ok(results)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_capture_folder() -> Result<(), String> {
    open_path(pictures_dir()?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn capture_view_path(state: State<'_, CaptureViewState>) -> Option<String> {
    state.path.lock().ok().and_then(|path| path.clone())
}

#[tauri::command]
pub async fn open_capture_view(
    app: AppHandle,
    state: State<'_, CaptureViewState>,
    path: String,
) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("Screenshot file is missing".into());
    }
    if let Ok(mut guard) = state.path.lock() {
        *guard = Some(path.clone());
    }
    if let Some(window) = app.get_webview_window("capture-view") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_fullscreen(true);
        let _ = window.set_focus();
        let _ = app.emit_to("capture-view", "capture-view-updated", ());
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "capture-view",
            tauri::WebviewUrl::App("capture-view.html".into()),
        )
        .title("Isle — Screenshot")
        .inner_size(1280.0, 800.0)
        .decorations(false)
        .fullscreen(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .background_color(tauri::window::Color(5, 7, 13, 255))
        .build()
        .map_err(|error| error.to_string())?;
        let _ = window.set_fullscreen(true);
        let _ = window.set_focus();
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())?
}

pub fn set_clipboard_image(image: image::RgbaImage) -> Result<(), String> {
    let (width, height) = image.dimensions();
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clipboard_write_image(data_url: String) -> Result<(), String> {
    let payload = data_url
        .split_once(',')
        .map(|(_, value)| value)
        .unwrap_or(data_url.as_str());
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|error| error.to_string())?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| error.to_string())?
        .to_rgba8();
    set_clipboard_image(image)
}

#[tauri::command]
pub fn clipboard_write_image_path(path: String) -> Result<(), String> {
    let image = image::open(Path::new(&path))
        .map_err(|error| error.to_string())?
        .to_rgba8();
    set_clipboard_image(image)
}

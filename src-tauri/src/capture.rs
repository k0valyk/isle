use std::borrow::Cow;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine as _;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::{FileAccessMode, StorageFile};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[link(name = "user32")]
extern "system" {
    fn GetDC(hwnd: isize) -> isize;
    fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn EnumDisplayMonitors(
        hdc: isize,
        clip: *const Rect,
        callback: MonitorEnumProc,
        data: isize,
    ) -> i32;
    fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
}

type MonitorEnumProc = Option<unsafe extern "system" fn(isize, isize, *mut Rect, isize) -> i32>;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: Rect,
    rc_work: Rect,
    flags: u32,
}

const MONITORINFOF_PRIMARY: u32 = 1;

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: isize) -> isize;
    fn CreateCompatibleBitmap(hdc: isize, width: i32, height: i32) -> isize;
    fn SelectObject(hdc: isize, object: isize) -> isize;
    fn BitBlt(
        dest: isize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        source: isize,
        source_x: i32,
        source_y: i32,
        rop: u32,
    ) -> i32;
    fn GetDIBits(
        hdc: isize,
        bitmap: isize,
        start: u32,
        lines: u32,
        bits: *mut u8,
        info: *mut BitmapInfo,
        usage: u32,
    ) -> i32;
    fn DeleteObject(object: isize) -> i32;
    fn DeleteDC(hdc: isize) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: isize,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    clr_used: u32,
    clr_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 3],
}

const SM_XVIRTUALSCREEN: i32 = 76;
const SM_YVIRTUALSCREEN: i32 = 77;
const SM_CXVIRTUALSCREEN: i32 = 78;
const SM_CYVIRTUALSCREEN: i32 = 79;
const SRCCOPY: u32 = 0x00CC_0020;
const CAPTUREBLT: u32 = 0x4000_0000;
const DIB_RGB_COLORS: u32 = 0;
const BI_RGB: u32 = 0;

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

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn pictures_dir() -> Result<PathBuf, String> {
    let dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(crate::settings::data_dir);
    let target = dir.join("Isle");
    std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;
    Ok(target)
}

fn preview_data_url(path: &Path) -> Result<String, String> {
    let image = image::open(path).map_err(|error| error.to_string())?;
    let thumb = image.thumbnail(760, 480);
    let mut buffer = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
    Ok(format!("data:image/png;base64,{encoded}"))
}

#[derive(Default)]
struct MonitorCollector {
    rects: Vec<(Rect, bool)>,
}

unsafe extern "system" fn monitor_callback(
    monitor: isize,
    _hdc: isize,
    _clip: *mut Rect,
    data: isize,
) -> i32 {
    let collector = &mut *(data as *mut MonitorCollector);
    let mut info = MonitorInfo {
        cb_size: std::mem::size_of::<MonitorInfo>() as u32,
        ..MonitorInfo::default()
    };
    if GetMonitorInfoW(monitor, &mut info) != 0 {
        collector
            .rects
            .push((info.rc_monitor, info.flags & MONITORINFOF_PRIMARY != 0));
    }
    1
}

fn monitors() -> Vec<CaptureMonitor> {
    let mut collector = MonitorCollector::default();
    unsafe {
        let _ = EnumDisplayMonitors(
            0,
            std::ptr::null(),
            Some(monitor_callback),
            &mut collector as *mut _ as isize,
        );
    }
    collector
        .rects
        .into_iter()
        .enumerate()
        .map(|(index, (rect, primary))| CaptureMonitor {
            index,
            name: format!("Display {}", index + 1),
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left).max(1) as u32,
            height: (rect.bottom - rect.top).max(1) as u32,
            primary,
        })
        .collect()
}

fn capture_to_rect(path: &Path, left: i32, top: i32, width: i32, height: i32) -> Result<(u32, u32), String> {
    unsafe {
        if width <= 0 || height <= 0 {
            return Err("Screen metrics are unavailable".into());
        }

        let screen = GetDC(0);
        if screen == 0 {
            return Err("Could not get the screen device context".into());
        }
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        if memory == 0 || bitmap == 0 {
            ReleaseDC(0, screen);
            return Err("Could not allocate a capture surface".into());
        }

        let previous = SelectObject(memory, bitmap);
        let copied = BitBlt(
            memory,
            0,
            0,
            width,
            height,
            screen,
            left,
            top,
            SRCCOPY | CAPTUREBLT,
        );
        if copied == 0 {
            SelectObject(memory, previous);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(0, screen);
            return Err("BitBlt failed to copy the screen".into());
        }

        let mut info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width,
                height,
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                size_image: 0,
                x_pels_per_meter: 0,
                y_pels_per_meter: 0,
                clr_used: 0,
                clr_important: 0,
            },
            colors: [0, 0, 0],
        };

        let stride = width as usize * 4;
        let mut buffer = vec![0u8; stride * height as usize];
        let lines = GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            buffer.as_mut_ptr(),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(memory, previous);
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(0, screen);

        if lines == 0 {
            return Err("GetDIBits failed to read pixels".into());
        }

        // GDI returns bottom-up BGRA rows; convert to top-down RGBA.
        let mut rgba = vec![0u8; buffer.len()];
        for row in 0..height as usize {
            let source = &buffer[(height as usize - 1 - row) * stride..][..stride];
            let target = &mut rgba[row * stride..][..stride];
            for column in 0..width as usize {
                let offset = column * 4;
                target[offset] = source[offset + 2];
                target[offset + 1] = source[offset + 1];
                target[offset + 2] = source[offset];
                target[offset + 3] = 255;
            }
        }

        let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)
            .ok_or_else(|| "Could not build the screenshot image".to_string())?;
        image.save(path).map_err(|error| error.to_string())?;

        Ok((width as u32, height as u32))
    }
}

fn capture_to(path: &Path) -> Result<(u32, u32), String> {
    unsafe {
        capture_to_rect(
            path,
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
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
    let keep_position = island
        .as_ref()
        .and_then(crate::window_util::physical_position);
    if was_visible {
        if let Some(window) = island.as_ref() {
            let _ = window.hide();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(160));
    let result = tauri::async_runtime::spawn_blocking(move || {
        let dir = pictures_dir()?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let name = format!("Isle-{stamp}.png");
        let path = dir.join(&name);
        let (width, height) = match monitor.and_then(|index| monitors().into_iter().find(|item| item.index == index)) {
            Some(target) => capture_to_rect(
                &path,
                target.x,
                target.y,
                target.width as i32,
                target.height as i32,
            )?,
            None => capture_to(&path)?,
        };
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
pub fn capture_monitors() -> Vec<CaptureMonitor> {
    monitors()
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
                let metadata = entry.metadata().ok();
                let modified = metadata
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
        let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
        let width = image.width();
        let height = image.height();

        let dir = pictures_dir()?;
        let fallback = format!(
            "Isle-edit-{}.png",
            chrono::Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let requested = name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback);
        let mut file_name: String = requested
            .chars()
            .filter(|character| {
                character.is_ascii_alphanumeric()
                    || *character == '.'
                    || *character == '-'
                    || *character == '_'
            })
            .collect();
        if file_name.is_empty() {
            file_name = "Isle-edit.png".into();
        }
        if !file_name.to_ascii_lowercase().ends_with(".png") {
            file_name.push_str(".png");
        }
        let path = dir.join(file_name);
        std::fs::write(&path, &bytes).map_err(|error| error.to_string())?;
        Ok(CaptureResult {
            preview: preview_data_url(&path)?,
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Isle-edit.png")
                .to_string(),
            width,
            height,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

fn ocr_sync(path: &Path) -> Result<String, String> {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let file = StorageFile::GetFileFromPathAsync(&windows::core::HSTRING::from(
        path.to_string_lossy().to_string(),
    ))
    .and_then(|operation| operation.join())
    .map_err(|error| format!("Open image failed: {error}"))?;
    let stream = file
        .OpenAsync(FileAccessMode::Read)
        .and_then(|operation| operation.join())
        .map_err(|error| format!("Read image failed: {error}"))?;
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .and_then(|operation| operation.join())
        .map_err(|error| format!("Decode image failed: {error}"))?;
    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .and_then(|operation| operation.join())
        .map_err(|error| format!("Bitmap conversion failed: {error}"))?;

    let engine = OcrEngine::TryCreateFromUserProfileLanguages().or_else(|_| {
        OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&windows::core::HSTRING::from(
            "en-US",
        ))?)
    })
        .map_err(|error| format!("OCR engine is unavailable: {error}"))?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .and_then(|operation| operation.join())
        .map_err(|error| format!("OCR failed: {error}"))?;
    let text = result
        .Text()
        .map(|value| value.to_string())
        .map_err(|error| format!("OCR text is unavailable: {error}"))?;
    Ok(text)
}

#[tauri::command]
pub async fn ocr_image(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || ocr_sync(Path::new(&path)))
        .await
        .map_err(|error| error.to_string())?
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
        let modules = code.width();
        let quiet = 4usize;
        let target = (size.unwrap_or(320).clamp(96, 1024) as usize).max(modules + quiet * 2);
        let cell = (target / (modules + quiet * 2)).max(1);
        let total = (modules + quiet * 2) * cell;
        let colors = code.to_colors();
        let mut image = image::GrayImage::from_pixel(total as u32, total as u32, image::Luma([255]));
        for y in 0..modules {
            for x in 0..modules {
                if colors[y * modules + x] == qrcode::Color::Dark {
                    for dy in 0..cell {
                        for dx in 0..cell {
                            image.put_pixel(
                                ((x + quiet) * cell + dx) as u32,
                                ((y + quiet) * cell + dy) as u32,
                                image::Luma([0]),
                            );
                        }
                    }
                }
            }
        }
        let mut buffer = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
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
        let mut result = Vec::new();
        for grid in grids {
            if let Ok((_meta, content)) = grid.decode() {
                if !content.trim().is_empty() && !result.contains(&content) {
                    result.push(content);
                }
            }
        }
        Ok(result)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let operation = wide(OsStr::new("open"));
    let file = wide(OsStr::new(&path));
    let result = unsafe {
        ShellExecuteW(
            0,
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result as usize <= 32 {
        return Err(format!("ShellExecute failed with code {result}"));
    }
    Ok(())
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
    edit: Option<bool>,
) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err("Screenshot file is missing".into());
    }
    let edit = edit.unwrap_or(false);
    if let Ok(mut guard) = state.path.lock() {
        *guard = Some(path.clone());
    }
    if let Some(window) = app.get_webview_window("capture-view") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_fullscreen(true);
        let _ = window.set_focus();
        let _ = app.emit_to("capture-view", "capture-view-updated", ());
        if edit {
            let _ = app.emit_to("capture-view", "capture-view-edit", ());
        }
        return Ok(());
    }
    // Create the fullscreen viewer only when it is first used: a permanently
    // loaded WebView2 window costs ~40 MB of private memory at all times.
    // Window creation must happen off the command caller's thread.
    tauri::async_runtime::spawn_blocking(move || {
        let url = if edit {
            "capture-view.html?edit=1"
        } else {
            "capture-view.html"
        };
        let window = tauri::WebviewWindowBuilder::new(
            &app,
            "capture-view",
            tauri::WebviewUrl::App(url.into()),
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

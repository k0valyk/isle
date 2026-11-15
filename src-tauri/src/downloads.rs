use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: u32,
    pub url: String,
    pub name: String,
    pub path: String,
    pub received: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
    #[serde(skip)]
    cancelled: bool,
}

#[derive(Default)]
pub struct DownloadState {
    items: Mutex<Vec<DownloadItem>>,
    next: AtomicU32,
}

fn download_dir() -> PathBuf {
    let dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(crate::settings::data_dir)
        .join("Isle");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn sanitize_name(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("download")
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric()
                || *character == '.'
                || *character == '-'
                || *character == '_'
                || *character == ' '
        })
        .collect::<String>();
    let trimmed = base.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "download".into()
    } else {
        trimmed.chars().take(140).collect()
    }
}

fn name_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or("download");
    sanitize_name(name)
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.join(name);
    if !path.exists() {
        return path;
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    for index in 1..1000 {
        let candidate = if extension.is_empty() {
            format!("{stem}-{index}")
        } else {
            format!("{stem}-{index}.{extension}")
        };
        path = dir.join(candidate);
        if !path.exists() {
            return path;
        }
    }
    path
}

fn update_item(app: &AppHandle, id: u32, update: impl FnOnce(&mut DownloadItem)) {
    let state = app.state::<DownloadState>();
    let Ok(mut items) = state.items.lock() else {
        return;
    };
    if let Some(item) = items.iter_mut().find(|item| item.id == id) {
        update(item);
        let snapshot = item.clone();
        drop(items);
        let _ = app.emit("download-progress", &snapshot);
    }
}

fn is_cancelled(app: &AppHandle, id: u32) -> bool {
    let state = app.state::<DownloadState>();
    state
        .items
        .lock()
        .ok()
        .and_then(|items| items.iter().find(|item| item.id == id).map(|item| item.cancelled))
        .unwrap_or(true)
}

#[tauri::command]
pub fn download_start(
    app: AppHandle,
    state: State<'_, DownloadState>,
    url: String,
    name: Option<String>,
) -> Result<u32, String> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|error| error.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Only http and https links are supported".into());
    }
    let file_name = sanitize_name(
        name.as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&name_from_url(parsed.as_str())),
    );
    let path = unique_path(&download_dir(), &file_name);
    let id = state.next.fetch_add(1, Ordering::Relaxed) + 1;
    let item = DownloadItem {
        id,
        url: parsed.to_string(),
        name: file_name,
        path: path.to_string_lossy().to_string(),
        received: 0,
        total: 0,
        done: false,
        error: None,
        cancelled: false,
    };
    state
        .items
        .lock()
        .map_err(|error| error.to_string())?
        .insert(0, item.clone());
    let _ = app.emit("download-progress", &item);

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_download(handle, id, parsed, path).await;
    });

    Ok(id)
}

async fn run_download(app: AppHandle, id: u32, url: reqwest::Url, path: PathBuf) {
    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .user_agent(format!("Isle/{}", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            update_item(&app, id, |item| {
                item.done = true;
                item.error = Some(error.to_string());
            });
            return;
        }
    };

    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => {
            update_item(&app, id, |item| {
                item.done = true;
                item.error = Some(error.to_string());
            });
            return;
        }
    };
    if !response.status().is_success() {
        update_item(&app, id, |item| {
            item.done = true;
            item.error = Some(format!("HTTP {}", response.status()));
        });
        return;
    }

    let total = response.content_length().unwrap_or(0);
    update_item(&app, id, |item| item.total = total);

    let mut file = match std::fs::File::create(&path) {
        Ok(file) => file,
        Err(error) => {
            update_item(&app, id, |item| {
                item.done = true;
                item.error = Some(error.to_string());
            });
            return;
        }
    };

    let mut stream = response.bytes_stream();
    let mut received = 0u64;
    let mut last_emit = Instant::now();
    let mut error: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        if is_cancelled(&app, id) {
            error = Some("cancelled".into());
            break;
        }
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(cause) => {
                error = Some(cause.to_string());
                break;
            }
        };
        if let Err(cause) = file.write_all(&chunk) {
            error = Some(cause.to_string());
            break;
        }
        received += chunk.len() as u64;
        if last_emit.elapsed() >= Duration::from_millis(120) {
            update_item(&app, id, |item| item.received = received);
            last_emit = Instant::now();
        }
    }

    let _ = file.flush();
    let failed = error.is_some();
    if failed {
        let _ = std::fs::remove_file(&path);
    }
    update_item(&app, id, |item| {
        item.received = received;
        item.done = true;
        item.error = error;
    });
    let _ = app.emit("download-done", id);
}

#[tauri::command]
pub fn downloads_list(state: State<'_, DownloadState>) -> Vec<DownloadItem> {
    state
        .items
        .lock()
        .map(|items| items.iter().take(40).cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn download_cancel(state: State<'_, DownloadState>, id: u32) {
    if let Ok(mut items) = state.items.lock() {
        if let Some(item) = items.iter_mut().find(|item| item.id == id) {
            item.cancelled = true;
        }
    }
}

#[tauri::command]
pub fn download_clear(state: State<'_, DownloadState>) {
    if let Ok(mut items) = state.items.lock() {
        items.retain(|item| !item.done);
    }
}

#[tauri::command]
pub fn downloads_open_folder() -> Result<(), String> {
    crate::capture::open_path(download_dir().to_string_lossy().to_string())
}

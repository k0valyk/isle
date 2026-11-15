use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

const PORT: u16 = 45872;
const MAX_FILE: usize = 50 * 1024 * 1024;
const MAX_JSON: usize = 128 * 1024;
const MOBILE_APP: &str = include_str!("phone_web.html");

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PhoneNotification {
    pub id: i64,
    pub app: String,
    pub title: String,
    pub text: String,
    pub ts: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PhoneFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub ts: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PhoneInfo {
    pub ip: String,
    pub port: u16,
    pub code: String,
    pub paired: bool,
    pub device: Option<String>,
    pub notifications: Vec<PhoneNotification>,
    pub files: Vec<PhoneFile>,
}

struct PhoneInner {
    code: String,
    token: Option<String>,
    device: Option<String>,
    ip: String,
    notifications: VecDeque<PhoneNotification>,
    files: VecDeque<PhoneFile>,
    next_id: i64,
}

pub struct PhoneState {
    inner: Mutex<PhoneInner>,
}

#[derive(Deserialize, Default)]
struct PairBody {
    #[serde(default)]
    code: String,
    #[serde(default)]
    device: String,
}

#[derive(Deserialize, Default)]
struct NotificationBody {
    #[serde(default)]
    app: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize, Default)]
struct ClipboardBody {
    #[serde(default)]
    text: String,
}

#[cfg(windows)]
fn local_ipv4() -> String {
    use std::os::windows::process::CommandExt;
    let script = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $ip = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } | Sort-Object InterfaceMetric | Select-Object -First 1 -ExpandProperty IPAddress; if ($ip) { $ip } else { '127.0.0.1' }"#;
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "127.0.0.1".into())
}

#[cfg(not(windows))]
fn local_ipv4() -> String {
    // `ipconfig getifaddr en0` returns the primary LAN address on macOS.
    if let Ok(output) = std::process::Command::new("ipconfig")
        .args(["getifaddr", "en0"])
        .output()
    {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !value.is_empty() {
            return value;
        }
    }
    "127.0.0.1".into()
}

fn download_dir() -> PathBuf {
    let dir = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(crate::settings::data_dir)
        .join("Isle Phone");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn sanitize_name(name: &str) -> String {
    let base = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("file")
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || *character == '.' || *character == '-' || *character == '_'
        })
        .collect::<String>();
    let trimmed = base.trim_matches('.').to_string();
    if trimmed.is_empty() {
        "file".into()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.join(name);
    if !path.exists() {
        return path;
    }
    let stem = std::path::Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let extension = std::path::Path::new(name)
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

fn query_value(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        if name == key {
            return Some(
                value
                    .replace("%20", " ")
                    .replace("%2F", "/")
                    .replace("%5C", "\\"),
            );
        }
    }
    None
}

struct Request {
    method: String,
    path: String,
    token: Option<String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 16384];
    let header_end = loop {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_FILE + MAX_JSON {
            return None;
        }
        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
    };

    let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();

    let mut content_length = 0usize;
    let mut token: Option<String> = query_value(&path, "token");
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            } else if name.eq_ignore_ascii_case("x-isle-token") {
                token = Some(value.trim().to_string());
            }
        }
    }
    if content_length > MAX_FILE {
        return None;
    }

    let mut body = buffer[header_end + 4..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Some(Request {
        method,
        path,
        token,
        body,
    })
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    respond_raw(
        stream,
        status,
        "application/json; charset=utf-8",
        body.as_bytes(),
    );
}

fn respond_raw(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn authorized(inner: &PhoneInner, token: &Option<String>) -> bool {
    match (&inner.token, token) {
        (Some(expected), Some(provided)) => expected == provided,
        _ => false,
    }
}

fn handle(mut stream: TcpStream, app: &AppHandle) {
    let Some(request) = read_request(&mut stream) else {
        respond(&mut stream, "400 Bad Request", "{\"error\":\"bad request\"}");
        return;
    };
    let route = request.path.split('?').next().unwrap_or(&request.path);
    let state = app.state::<PhoneState>();
    let mut inner = match state.inner.lock() {
        Ok(inner) => inner,
        Err(_) => {
            respond(&mut stream, "500 Internal Server Error", "{\"error\":\"state\"}");
            return;
        }
    };

    match (request.method.as_str(), route) {
        ("GET", "/") | ("GET", "/app") => {
            respond_raw(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                MOBILE_APP.as_bytes(),
            );
        }
        ("GET", "/health") | ("GET", "/info") => {
            respond(
                &mut stream,
                "200 OK",
                &json!({
                    "app": "isle",
                    "version": env!("CARGO_PKG_VERSION"),
                    "paired": inner.token.is_some(),
                })
                .to_string(),
            );
        }
        ("GET", "/clipboard") => {
            if !authorized(&inner, &request.token) {
                respond(&mut stream, "401 Unauthorized", "{\"error\":\"pair first\"}");
                return;
            }
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
                Ok(text) => respond(
                    &mut stream,
                    "200 OK",
                    &json!({ "text": text }).to_string(),
                ),
                Err(_) => respond(&mut stream, "200 OK", "{\"text\":\"\"}"),
            };
        }
        ("POST", "/clipboard") => {
            if !authorized(&inner, &request.token) {
                respond(&mut stream, "401 Unauthorized", "{\"error\":\"pair first\"}");
                return;
            }
            let body: ClipboardBody = match serde_json::from_slice(&request.body) {
                Ok(body) => body,
                Err(_) => {
                    respond(&mut stream, "400 Bad Request", "{\"error\":\"invalid json\"}");
                    return;
                }
            };
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(body.text)) {
                Ok(()) => respond(&mut stream, "200 OK", "{\"ok\":true}"),
                Err(error) => respond(
                    &mut stream,
                    "500 Internal Server Error",
                    &json!({ "error": error.to_string() }).to_string(),
                ),
            };
        }
        ("POST", "/pair") => {
            let body: PairBody = match serde_json::from_slice(&request.body) {
                Ok(body) => body,
                Err(_) => {
                    respond(&mut stream, "400 Bad Request", "{\"error\":\"invalid json\"}");
                    return;
                }
            };
            if body.code.trim() != inner.code {
                respond(&mut stream, "403 Forbidden", "{\"error\":\"wrong pairing code\"}");
                return;
            }
            let token = Uuid::new_v4().simple().to_string();
            inner.token = Some(token.clone());
            inner.device = Some(if body.device.trim().is_empty() {
                "Android".into()
            } else {
                body.device.chars().take(60).collect()
            });
            let device = inner.device.clone();
            let _ = app.emit("phone-paired", &device);
            respond(&mut stream, "200 OK", &json!({ "token": token }).to_string());
        }
        ("POST", "/notification") => {
            if !authorized(&inner, &request.token) {
                respond(&mut stream, "401 Unauthorized", "{\"error\":\"pair first\"}");
                return;
            }
            let body: NotificationBody = match serde_json::from_slice(&request.body) {
                Ok(body) => body,
                Err(_) => {
                    respond(&mut stream, "400 Bad Request", "{\"error\":\"invalid json\"}");
                    return;
                }
            };
            inner.next_id += 1;
            let notification = PhoneNotification {
                id: inner.next_id,
                app: body.app.chars().take(60).collect(),
                title: body.title.chars().take(160).collect(),
                text: body.text.chars().take(500).collect(),
                ts: chrono::Local::now().timestamp_millis(),
            };
            inner.notifications.push_front(notification.clone());
            while inner.notifications.len() > 120 {
                inner.notifications.pop_back();
            }
            let _ = app.emit("phone-notification", &notification);
            let _ = app
                .notification()
                .builder()
                .title(if notification.title.is_empty() {
                    notification.app.clone()
                } else {
                    notification.title.clone()
                })
                .body(notification.text.clone())
                .show();
            respond(&mut stream, "200 OK", "{\"ok\":true}");
        }
        ("POST", "/file") => {
            if !authorized(&inner, &request.token) {
                respond(&mut stream, "401 Unauthorized", "{\"error\":\"pair first\"}");
                return;
            }
            let requested = query_value(&request.path, "name").unwrap_or_else(|| "file".into());
            let name = sanitize_name(&requested);
            let dir = download_dir();
            let path = unique_path(&dir, &name);
            if std::fs::write(&path, &request.body).is_err() {
                respond(&mut stream, "500 Internal Server Error", "{\"error\":\"write failed\"}");
                return;
            }
            let file = PhoneFile {
                name: path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("file")
                    .to_string(),
                path: path.to_string_lossy().to_string(),
                size: request.body.len() as u64,
                ts: chrono::Local::now().timestamp_millis(),
            };
            inner.files.push_front(file.clone());
            while inner.files.len() > 120 {
                inner.files.pop_back();
            }
            let _ = app.emit("phone-file", &file);
            respond(
                &mut stream,
                "200 OK",
                &json!({ "ok": true, "name": file.name }).to_string(),
            );
        }
        ("GET", "/files") => {
            if !authorized(&inner, &request.token) {
                respond(&mut stream, "401 Unauthorized", "{\"error\":\"pair first\"}");
                return;
            }
            let files: Vec<&PhoneFile> = inner.files.iter().collect();
            respond(
                &mut stream,
                "200 OK",
                &serde_json::to_string(&files).unwrap_or_else(|_| "[]".into()),
            );
        }
        _ => {
            respond(
                &mut stream,
                "404 Not Found",
                "{\"error\":\"use /info, /pair, /notification, /file or /files\"}",
            );
        }
    }
}

fn make_code() -> String {
    let value = Uuid::new_v4().as_u128() % 1_000_000;
    format!("{value:06}")
}

pub fn init(app: &AppHandle) {
    app.manage(PhoneState {
        inner: Mutex::new(PhoneInner {
            code: make_code(),
            token: None,
            device: None,
            ip: local_ipv4(),
            notifications: VecDeque::new(),
            files: VecDeque::new(),
            next_id: 0,
        }),
    });

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(("0.0.0.0", PORT)) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Isle: phone server unavailable on port {PORT}: {error}");
                return;
            }
        };
        eprintln!("Isle: phone pairing server listening on port {PORT}");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let connection_app = app_handle.clone();
                    std::thread::spawn(move || handle(stream, &connection_app));
                }
                Err(error) => eprintln!("Isle: phone connection failed: {error}"),
            }
        }
    });
}

#[tauri::command]
pub fn phone_info(state: State<'_, PhoneState>) -> Result<PhoneInfo, String> {
    let inner = state.inner.lock().map_err(|error| error.to_string())?;
    Ok(PhoneInfo {
        ip: inner.ip.clone(),
        port: PORT,
        code: inner.code.clone(),
        paired: inner.token.is_some(),
        device: inner.device.clone(),
        notifications: inner.notifications.iter().cloned().collect(),
        files: inner.files.iter().cloned().collect(),
    })
}

#[tauri::command]
pub fn phone_unpair(state: State<'_, PhoneState>, app: AppHandle) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|error| error.to_string())?;
    inner.token = None;
    inner.device = None;
    inner.code = make_code();
    let _ = app.emit("phone-unpaired", ());
    Ok(())
}

#[tauri::command]
pub fn phone_refresh_code(state: State<'_, PhoneState>) -> Result<String, String> {
    let mut inner = state.inner.lock().map_err(|error| error.to_string())?;
    inner.code = make_code();
    inner.token = None;
    inner.device = None;
    Ok(inner.code.clone())
}

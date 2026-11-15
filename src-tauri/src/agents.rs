use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

const ENDPOINT: &str = "127.0.0.1:45871";
const MAX_BODY: usize = 256 * 1024;
const MAX_EVENTS: usize = 120;
const OPENCODE_PLUGIN: &str = include_str!("../../integrations/opencode/isle/index.ts");

#[derive(Serialize, Clone)]
pub struct AgentEvent {
    pub id: i64,
    pub source: String,
    pub event: String,
    pub title: String,
    pub detail: String,
    pub ts: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentsStatus {
    pub listening: bool,
    pub endpoint: String,
    pub requests: u64,
    pub started_at: i64,
    pub last_at: i64,
    pub opencode_plugin: bool,
    pub opencode_path: Option<String>,
}

pub struct AgentsState {
    events: Mutex<VecDeque<AgentEvent>>,
    next_id: AtomicI64,
    listening: AtomicBool,
    requests: AtomicU64,
    started_at: i64,
    last_at: AtomicI64,
}

impl Default for AgentsState {
    fn default() -> Self {
        Self {
            events: Mutex::new(VecDeque::new()),
            next_id: AtomicI64::new(0),
            listening: AtomicBool::new(false),
            requests: AtomicU64::new(0),
            started_at: chrono::Local::now().timestamp_millis(),
            last_at: AtomicI64::new(0),
        }
    }
}

#[derive(Deserialize, Default)]
struct Incoming {
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    detail: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

impl AgentsState {
    fn push(&self, incoming: Incoming) -> Option<AgentEvent> {
        let source = incoming
            .source
            .unwrap_or_else(|| "agent".into())
            .chars()
            .take(40)
            .collect::<String>();
        let event_name = incoming
            .event
            .unwrap_or_else(|| "notice".into())
            .chars()
            .take(40)
            .collect::<String>();
        let title = incoming
            .title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("{source}: {event_name}"))
            .chars()
            .take(160)
            .collect::<String>();
        let detail = incoming
            .detail
            .or(incoming.message)
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect::<String>();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let event = AgentEvent {
            id,
            source,
            event: event_name,
            title,
            detail,
            ts: chrono::Local::now().timestamp_millis(),
        };
        self.last_at.store(event.ts, Ordering::Relaxed);
        if let Ok(mut events) = self.events.lock() {
            if let Some(front) = events.front_mut() {
                let same_kind = front.source == event.source
                    && front.event == event.event
                    && front.title == event.title;
                let same_detail = front.detail == event.detail;
                let window = if event.event == "hook" { 4_000 } else { 1_500 };
                if same_kind && (same_detail || event.event == "hook") && event.ts - front.ts < window
                {
                    // OpenCode loads the plugin once per workspace, so the same
                    // session event can arrive three times at once. Keep one row.
                    front.ts = event.ts;
                    front.id = front.id.max(event.id);
                    return None;
                }
            }
            events.push_front(event.clone());
            while events.len() > MAX_EVENTS {
                events.pop_back();
            }
        }
        Some(event)
    }
}

#[tauri::command]
pub fn agents_events(state: State<'_, AgentsState>) -> Vec<AgentEvent> {
    state
        .events
        .lock()
        .map(|events| events.iter().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn agents_clear(state: State<'_, AgentsState>) {
    if let Ok(mut events) = state.events.lock() {
        events.clear();
    }
}

#[tauri::command]
pub fn agents_endpoint() -> String {
    ENDPOINT.to_string()
}

fn opencode_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut push = |path: PathBuf| {
        if !dirs.iter().any(|item| item == &path) {
            dirs.push(path);
        }
    };
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            push(PathBuf::from(xdg).join("opencode").join("plugins").join("isle"));
        }
    }
    if let Some(base) = directories::BaseDirs::new() {
        push(
            base.home_dir()
                .join(".config")
                .join("opencode")
                .join("plugins")
                .join("isle"),
        );
        let appdata = base.config_dir().join("opencode");
        if appdata.exists() {
            push(appdata.join("plugins").join("isle"));
        }
    }
    if dirs.is_empty() {
        dirs.push(PathBuf::from(".config/opencode/plugins/isle"));
    }
    dirs
}

fn opencode_files() -> Vec<PathBuf> {
    opencode_dirs()
        .into_iter()
        .map(|dir| dir.join("index.ts"))
        .collect()
}

fn opencode_file() -> Option<PathBuf> {
    let files = opencode_files();
    files
        .iter()
        .find(|path| path.exists())
        .cloned()
        .or_else(|| files.into_iter().next())
}

#[tauri::command]
pub fn agents_status(state: State<'_, AgentsState>) -> AgentsStatus {
    let last = state
        .events
        .lock()
        .ok()
        .and_then(|events| events.front().map(|event| event.ts))
        .unwrap_or(0);
    let files = opencode_files();
    let existing = files.iter().find(|path| path.exists()).cloned();
    AgentsStatus {
        listening: state.listening.load(Ordering::Relaxed),
        endpoint: ENDPOINT.to_string(),
        requests: state.requests.load(Ordering::Relaxed),
        started_at: state.started_at,
        last_at: state.last_at.load(Ordering::Relaxed).max(last),
        opencode_plugin: existing.is_some(),
        opencode_path: existing
            .or_else(|| files.into_iter().next())
            .map(|path| path.to_string_lossy().to_string()),
    }
}

#[tauri::command]
pub fn agents_test(
    app: AppHandle,
    state: State<'_, AgentsState>,
) -> Option<AgentEvent> {
    state.requests.fetch_add(1, Ordering::Relaxed);
    let event = state.push(Incoming {
        source: Some("opencode".into()),
        event: Some("test".into()),
        title: Some("Isle hook test".into()),
        detail: Some(format!("Hook test at {}", chrono::Local::now().format("%H:%M:%S"))),
        message: None,
    });
    if let Some(event) = event.as_ref() {
        let _ = app.emit("agent-event", event.clone());
    }
    let _ = app.emit("agents-status", agents_status(state));
    event
}

#[tauri::command]
pub fn agents_install_opencode(app: AppHandle, state: State<'_, AgentsState>) -> Result<String, String> {
    let mut written = Vec::new();
    for path in opencode_files() {
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                continue;
            }
        }
        if std::fs::write(&path, OPENCODE_PLUGIN).is_ok() {
            written.push(path.to_string_lossy().to_string());
        }
    }
    if written.is_empty() {
        return Err("Could not write the OpenCode plugin".into());
    }
    let event = state.push(Incoming {
        source: Some("opencode".into()),
        event: Some("installed".into()),
        title: Some("OpenCode plugin installed".into()),
        detail: Some(written.join(" · ")),
        message: None,
    });
    if let Some(event) = event {
        let _ = app.emit("agent-event", event);
    }
    let _ = app.emit("agents-status", agents_status(state));
    Ok(written.join(" · "))
}

#[tauri::command]
pub fn agents_open_opencode() -> Result<(), String> {
    let path = opencode_file().ok_or_else(|| "Could not resolve the config directory".to_string())?;
    let folder = if path.exists() {
        path.parent().unwrap_or(&path).to_path_buf()
    } else {
        path.parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or(path)
    };
    crate::capture::open_path(folder.to_string_lossy().to_string())
        .map_err(|error| error.to_string())
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn read_request(stream: &mut TcpStream) -> Option<(String, String, Vec<u8>)> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    let header_end = loop {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_BODY + 16 * 1024 {
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
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    if content_length > MAX_BODY {
        return None;
    }

    let body_start = header_end + 4;
    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).ok()?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Some((method, path, body))
}

fn handle(mut stream: TcpStream, app: &AppHandle) {
    let state = app.state::<AgentsState>();
    state.requests.fetch_add(1, Ordering::Relaxed);
    let Some((method, path, body)) = read_request(&mut stream) else {
        respond(&mut stream, "400 Bad Request", "{\"error\":\"bad request\"}");
        return;
    };

    let route = path.split('?').next().unwrap_or(&path);
    match (method.as_str(), route) {
        ("GET", "/health") => {
            respond(
                &mut stream,
                "200 OK",
                "{\"ok\":true,\"app\":\"isle\",\"version\":\"0.1.0\"}",
            );
        }
        ("POST", "/event") | ("POST", "/") => {
            let incoming: Incoming = match serde_json::from_slice(&body) {
                Ok(value) => value,
                Err(error) => {
                    respond(
                        &mut stream,
                        "400 Bad Request",
                        &format!("{{\"error\":\"invalid json: {error}\"}}"),
                    );
                    return;
                }
            };
            let event = state.push(incoming);
            if let Some(event) = event {
                let _ = app.emit("agent-event", event);
            }
            let _ = app.emit("agents-status", agents_status(state));
            respond(&mut stream, "200 OK", "{\"ok\":true}");
        }
        ("POST", "/clear") => {
            if let Ok(mut events) = state.events.lock() {
                events.clear();
            }
            let _ = app.emit("agents-cleared", ());
            let _ = app.emit("agents-status", agents_status(state));
            respond(&mut stream, "200 OK", "{\"ok\":true}");
        }
        _ => {
            respond(
                &mut stream,
                "404 Not Found",
                "{\"error\":\"use GET /health or POST /event\"}",
            );
        }
    }
}

pub fn spawn_server(app: AppHandle) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind(ENDPOINT) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("Isle: agent hook listener unavailable on {ENDPOINT}: {error}");
                return;
            }
        };
        app.state::<AgentsState>()
            .listening
            .store(true, Ordering::Relaxed);
        let _ = app.emit("agents-status", agents_status(app.state::<AgentsState>()));
        eprintln!("Isle: agent hooks listening on http://{ENDPOINT}");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handle_app = app.clone();
                    std::thread::spawn(move || handle(stream, &handle_app));
                }
                Err(error) => eprintln!("Isle: agent hook connection failed: {error}"),
            }
        }
    });
}

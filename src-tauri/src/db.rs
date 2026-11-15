use base64::Engine as _;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State};

const MAX_IMAGE_EDGE: u32 = 4096;

const CLIPBOARD_SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
CREATE TABLE IF NOT EXISTS clipboard (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL DEFAULT 'text',
    content TEXT NOT NULL,
    preview TEXT,
    source_app TEXT,
    ts INTEGER NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS clipboard_ts ON clipboard(ts DESC);
CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    content TEXT NOT NULL DEFAULT '',
    updated INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS notes_updated ON notes(updated DESC);
CREATE TABLE IF NOT EXISTS reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    due INTEGER NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    notified INTEGER NOT NULL DEFAULT 0,
    created INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS reminders_due ON reminders(due);
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    start INTEGER NOT NULL,
    end INTEGER NOT NULL,
    all_day INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS events_start ON events(start);
CREATE TABLE IF NOT EXISTS chat_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL DEFAULT '',
    created INTEGER NOT NULL,
    updated INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS chat_messages_session ON chat_messages(session_id, id);
";

pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

#[derive(Serialize, Clone)]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub preview: Option<String>,
    pub source_app: Option<String>,
    pub ts: i64,
    pub pinned: bool,
}

#[derive(Serialize, Clone)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub updated: i64,
}

#[derive(Deserialize)]
pub struct NoteInput {
    pub id: Option<i64>,
    pub title: String,
    pub content: String,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn image_dir() -> PathBuf {
    let dir = crate::settings::data_dir().join("clipboard-images");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn looks_secret(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < 12 {
        return false;
    }
    if trimmed.contains("BEGIN") && trimmed.contains("PRIVATE KEY") {
        return true;
    }
    if trimmed.to_lowercase().starts_with("otpauth://") {
        return true;
    }
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    for prefix in ["sk-", "ghp_", "gho_", "ghu_", "ghs_", "github_pat_", "xoxb-", "xoxp-"] {
        if compact.starts_with(prefix) && compact.len() > 24 {
            return true;
        }
    }
    if compact.starts_with("AKIA") && compact.len() >= 20 {
        return true;
    }
    let dots = compact.matches('.').count();
    if dots == 2 && compact.len() > 60 {
        let segments: Vec<&str> = compact.split('.').collect();
        if segments.len() == 3
            && segments[0].len() > 8
            && segments[1].len() > 8
            && segments[0]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return true;
        }
    }
    false
}

fn image_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn persist_image(width: u32, height: u32, bytes: &[u8]) -> Result<(String, String), String> {
    if width == 0 || height == 0 || width > MAX_IMAGE_EDGE || height > MAX_IMAGE_EDGE {
        return Err("Unsupported clipboard image size".into());
    }
    let buffer = image::RgbaImage::from_raw(width, height, bytes.to_vec())
        .ok_or_else(|| "Invalid clipboard image".to_string())?;
    let name = format!("clip-{}.png", now_ms());
    let path = image_dir().join(&name);
    image::DynamicImage::ImageRgba8(buffer)
        .save(&path)
        .map_err(|error| error.to_string())?;
    Ok((
        path.to_string_lossy().to_string(),
        format!("Зображення {width}×{height}"),
    ))
}

impl Database {
    pub fn open() -> Result<Self, String> {
        let path = crate::settings::data_dir().join("isle.db");
        let conn = Connection::open(path).map_err(|error| error.to_string())?;
        conn.execute_batch(CLIPBOARD_SCHEMA)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn insert_clipboard(&self, text: &str) -> Result<(), String> {
        let guard = self.conn.lock().map_err(|error| error.to_string())?;
        let last: Option<String> = guard
            .query_row(
                "SELECT content FROM clipboard ORDER BY ts DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        if last.as_deref() == Some(text) {
            guard
                .execute(
                    "UPDATE clipboard SET ts = ?1 WHERE id = (SELECT id FROM clipboard ORDER BY ts DESC LIMIT 1)",
                    params![now_ms()],
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        let preview: String = text.chars().take(200).collect::<String>().replace('\n', " ");
        guard
            .execute(
                "INSERT INTO clipboard (kind, content, preview, source_app, ts, pinned) VALUES ('text', ?1, ?2, NULL, ?3, 0)",
                params![text, preview, now_ms()],
            )
            .map_err(|error| error.to_string())?;
        self.trim(&guard)?;
        Ok(())
    }

    fn insert_item(
        &self,
        kind: &str,
        content: &str,
        preview: &str,
        source_app: Option<&str>,
    ) -> Result<(), String> {
        let guard = self.conn.lock().map_err(|error| error.to_string())?;
        let last: Option<String> = guard
            .query_row(
                "SELECT content FROM clipboard ORDER BY ts DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok();
        if last.as_deref() == Some(content) {
            guard
                .execute(
                    "UPDATE clipboard SET ts = ?1, preview = ?2 WHERE id = (SELECT id FROM clipboard ORDER BY ts DESC LIMIT 1)",
                    params![now_ms(), preview],
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        guard
            .execute(
                "INSERT INTO clipboard (kind, content, preview, source_app, ts, pinned) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![kind, content, preview, source_app, now_ms()],
            )
            .map_err(|error| error.to_string())?;
        self.trim(&guard)?;
        Ok(())
    }

    fn trim(&self, guard: &Connection) -> Result<(), String> {
        // Keep pinned rows and the newest 800 unpinned rows. Images are stored
        // as files, so remove the files of rows that are dropped.
        let mut stmt = guard
            .prepare(
                "SELECT id, content, kind FROM clipboard WHERE pinned = 0 \
                 AND id NOT IN (SELECT id FROM clipboard WHERE pinned = 0 ORDER BY ts DESC LIMIT 800)",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let dropped = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        drop(stmt);
        for (id, content, kind) in dropped {
            if kind == "image" {
                let _ = std::fs::remove_file(content);
            }
            guard
                .execute("DELETE FROM clipboard WHERE id = ?1", params![id])
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn row_to_clip(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipItem> {
    Ok(ClipItem {
        id: row.get(0)?,
        kind: row.get(1)?,
        content: row.get(2)?,
        preview: row.get(3)?,
        source_app: row.get(4)?,
        ts: row.get(5)?,
        pinned: row.get::<_, i64>(6)? != 0,
    })
}

fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        updated: row.get(3)?,
    })
}

#[tauri::command]
pub fn clipboard_list(
    state: State<'_, crate::AppState>,
    limit: Option<i64>,
) -> Result<Vec<ClipItem>, String> {
    let limit = limit.unwrap_or(200).clamp(1, 2000);
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT id, kind, content, preview, source_app, ts, pinned \
             FROM clipboard ORDER BY pinned DESC, ts DESC LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![limit], row_to_clip)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clipboard_search(
    state: State<'_, crate::AppState>,
    q: String,
) -> Result<Vec<ClipItem>, String> {
    let needle = format!("%{}%", q.trim());
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT id, kind, content, preview, source_app, ts, pinned \
             FROM clipboard WHERE content LIKE ?1 ESCAPE '\\' \
             ORDER BY pinned DESC, ts DESC LIMIT 300",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![needle], row_to_clip)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clipboard_pin(
    state: State<'_, crate::AppState>,
    id: i64,
    pinned: bool,
) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute(
            "UPDATE clipboard SET pinned = ?1 WHERE id = ?2",
            params![if pinned { 1 } else { 0 }, id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clipboard_delete(state: State<'_, crate::AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let image = guard
        .query_row(
            "SELECT kind, content FROM clipboard WHERE id = ?1",
            params![id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok();
    guard
        .execute("DELETE FROM clipboard WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    drop(guard);
    if let Some((kind, path)) = image {
        if kind == "image" {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn clipboard_capture(state: State<'_, crate::AppState>) -> Result<bool, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    let text = match clipboard.get_text() {
        Ok(text) => text,
        Err(_) => return Ok(false),
    };
    if text.trim().is_empty() {
        return Ok(false);
    }
    state.db.insert_clipboard(&text)?;
    Ok(true)
}

#[tauri::command]
pub fn clipboard_write(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clipboard_preview_data(state: State<'_, crate::AppState>, id: i64) -> Option<String> {
    let guard = state.db.conn.lock().ok()?;
    let (kind, content): (String, String) = guard
        .query_row(
            "SELECT kind, content FROM clipboard WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok()?;
    if kind != "image" {
        return None;
    }
    let bytes = std::fs::read(content).ok()?;
    let mime = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:{mime};base64,{encoded}"))
}

#[tauri::command]
pub fn clipboard_copy_item(state: State<'_, crate::AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let (kind, content): (String, String) = guard
        .query_row(
            "SELECT kind, content FROM clipboard WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    drop(guard);
    if kind == "image" {
        let decoded = image::open(&content).map_err(|error| error.to_string())?.to_rgba8();
        let (width, height) = decoded.dimensions();
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        clipboard
            .set_image(arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(decoded.into_raw()),
            })
            .map_err(|error| error.to_string())?;
        Ok(())
    } else {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        clipboard.set_text(content).map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub fn clipboard_clear(state: State<'_, crate::AppState>, keep_pinned: bool) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    if keep_pinned {
        let mut stmt = guard
            .prepare("SELECT content FROM clipboard WHERE pinned = 0 AND kind = 'image'")
            .map_err(|error| error.to_string())?;
        let paths = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        drop(stmt);
        for path in paths {
            let _ = std::fs::remove_file(path);
        }
        guard
            .execute("DELETE FROM clipboard WHERE pinned = 0", [])
            .map_err(|error| error.to_string())?;
    } else {
        let mut stmt = guard
            .prepare("SELECT content FROM clipboard WHERE kind = 'image'")
            .map_err(|error| error.to_string())?;
        let paths = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        drop(stmt);
        for path in paths {
            let _ = std::fs::remove_file(path);
        }
        guard
            .execute("DELETE FROM clipboard", [])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Poll the Windows clipboard in the background. This is what makes copied
/// text and images appear in history without pressing "Capture" every time.
pub fn spawn_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut clipboard = None;
        let mut last_text: Option<String> = None;
        let mut last_image: Option<u64> = None;
        let mut failures = 0u32;
        loop {
            std::thread::sleep(Duration::from_millis(650));
            let settings = app
                .state::<crate::AppState>()
                .settings
                .lock()
                .map(|guard| {
                    (
                        guard.clipboard_watch,
                        guard.clipboard_images,
                        guard.clipboard_ignore_secrets,
                    )
                })
                .ok();
            let Some((watch, images, ignore_secrets)) = settings else {
                continue;
            };
            if !watch {
                last_text = None;
                last_image = None;
                continue;
            }
            if clipboard.is_none() {
                match arboard::Clipboard::new() {
                    Ok(instance) => clipboard = Some(instance),
                    Err(_) => {
                        failures += 1;
                        if failures % 20 == 0 {
                            eprintln!("Isle: clipboard unavailable, retrying");
                        }
                        continue;
                    }
                }
            }
            let Some(instance) = clipboard.as_mut() else {
                continue;
            };
            let mut captured = false;
            if let Ok(text) = instance.get_text() {
                if !text.trim().is_empty() && last_text.as_deref() != Some(text.as_str()) {
                    last_text = Some(text.clone());
                    if !(ignore_secrets && looks_secret(&text)) {
                        let preview: String =
                            text.chars().take(200).collect::<String>().replace('\n', " ");
                        let result = app
                            .state::<crate::AppState>()
                            .db
                            .insert_item("text", &text, &preview, None);
                        if result.is_ok() {
                            captured = true;
                        }
                    }
                }
            } else if images {
                if let Ok(image) = instance.get_image() {
                    let hash = image_hash(&image.bytes);
                    if last_image != Some(hash) {
                        last_image = Some(hash);
                        if let Ok((path, preview)) =
                            persist_image(image.width as u32, image.height as u32, &image.bytes)
                        {
                            let result = app.state::<crate::AppState>().db.insert_item(
                                "image",
                                &path,
                                &preview,
                                None,
                            );
                            if result.is_ok() {
                                captured = true;
                            }
                        }
                    }
                }
            }
            if captured {
                let _ = app.emit("clipboard-changed", ());
            }
        }
    });
}

#[tauri::command]
pub fn notes_list(state: State<'_, crate::AppState>) -> Result<Vec<Note>, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare("SELECT id, title, content, updated FROM notes ORDER BY updated DESC")
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], row_to_note)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn notes_save(state: State<'_, crate::AppState>, note: NoteInput) -> Result<i64, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    match note.id {
        Some(id) => {
            guard
                .execute(
                    "UPDATE notes SET title = ?1, content = ?2, updated = ?3 WHERE id = ?4",
                    params![note.title, note.content, now_ms(), id],
                )
                .map_err(|error| error.to_string())?;
            Ok(id)
        }
        None => {
            guard
                .execute(
                    "INSERT INTO notes (title, content, updated) VALUES (?1, ?2, ?3)",
                    params![note.title, note.content, now_ms()],
                )
                .map_err(|error| error.to_string())?;
            Ok(guard.last_insert_rowid())
        }
    }
}

#[tauri::command]
pub fn notes_delete(state: State<'_, crate::AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute("DELETE FROM notes WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

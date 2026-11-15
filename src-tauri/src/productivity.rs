use chrono::{Local, NaiveDate, NaiveDateTime, TimeZone};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::AppState;

#[derive(Serialize, Clone)]
pub struct Reminder {
    pub id: i64,
    pub title: String,
    pub due: i64,
    pub done: bool,
    pub notified: bool,
    pub created: i64,
}

#[derive(Deserialize)]
pub struct ReminderInput {
    pub id: Option<i64>,
    pub title: String,
    pub due: i64,
}

#[derive(Serialize, Clone)]
pub struct CalendarEvent {
    pub id: i64,
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub notes: String,
}

#[derive(Deserialize)]
pub struct EventInput {
    pub id: Option<i64>,
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    #[serde(default)]
    pub notes: String,
}

fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

fn row_to_reminder(row: &rusqlite::Row<'_>) -> rusqlite::Result<Reminder> {
    Ok(Reminder {
        id: row.get(0)?,
        title: row.get(1)?,
        due: row.get(2)?,
        done: row.get::<_, i64>(3)? != 0,
        notified: row.get::<_, i64>(4)? != 0,
        created: row.get(5)?,
    })
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get(0)?,
        title: row.get(1)?,
        start: row.get(2)?,
        end: row.get(3)?,
        all_day: row.get::<_, i64>(4)? != 0,
        notes: row.get(5)?,
    })
}

#[tauri::command]
pub fn reminders_list(state: State<'_, AppState>) -> Result<Vec<Reminder>, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT id, title, due, done, notified, created FROM reminders \
             ORDER BY done ASC, due ASC LIMIT 500",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], row_to_reminder)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reminders_save(
    state: State<'_, AppState>,
    reminder: ReminderInput,
) -> Result<i64, String> {
    let title = reminder.title.trim();
    if title.is_empty() {
        return Err("Reminder title is empty".into());
    }
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    match reminder.id {
        Some(id) => {
            guard
                .execute(
                    "UPDATE reminders SET title = ?1, due = ?2, notified = 0 WHERE id = ?3",
                    params![title, reminder.due, id],
                )
                .map_err(|error| error.to_string())?;
            Ok(id)
        }
        None => {
            guard
                .execute(
                    "INSERT INTO reminders (title, due, done, notified, created) VALUES (?1, ?2, 0, 0, ?3)",
                    params![title, reminder.due, now_ms()],
                )
                .map_err(|error| error.to_string())?;
            Ok(guard.last_insert_rowid())
        }
    }
}

#[tauri::command]
pub fn reminders_toggle(state: State<'_, AppState>, id: i64, done: bool) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute(
            "UPDATE reminders SET done = ?1, notified = 1 WHERE id = ?2",
            params![if done { 1 } else { 0 }, id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reminders_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute("DELETE FROM reminders WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn events_list(
    state: State<'_, AppState>,
    from: i64,
    to: i64,
) -> Result<Vec<CalendarEvent>, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT id, title, start, end, all_day, notes FROM events \
             WHERE end >= ?1 AND start <= ?2 ORDER BY start ASC LIMIT 1000",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![from, to], row_to_event)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn events_save(state: State<'_, AppState>, event: EventInput) -> Result<i64, String> {
    let title = event.title.trim();
    if title.is_empty() {
        return Err("Event title is empty".into());
    }
    if event.end < event.start {
        return Err("Event end is before its start".into());
    }
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    match event.id {
        Some(id) => {
            guard
                .execute(
                    "UPDATE events SET title = ?1, start = ?2, end = ?3, all_day = ?4, notes = ?5 WHERE id = ?6",
                    params![title, event.start, event.end, if event.all_day { 1 } else { 0 }, event.notes, id],
                )
                .map_err(|error| error.to_string())?;
            Ok(id)
        }
        None => {
            guard
                .execute(
                    "INSERT INTO events (title, start, end, all_day, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![title, event.start, event.end, if event.all_day { 1 } else { 0 }, event.notes],
                )
                .map_err(|error| error.to_string())?;
            Ok(guard.last_insert_rowid())
        }
    }
}

#[tauri::command]
pub fn events_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute("DELETE FROM events WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn parse_ics_time(value: &str) -> Option<i64> {
    let value = value.trim();
    if value.len() == 8 {
        let date = NaiveDate::parse_from_str(value, "%Y%m%d").ok()?;
        let naive = date.and_hms_opt(0, 0, 0)?;
        return Local
            .from_local_datetime(&naive)
            .single()
            .map(|time| time.timestamp_millis());
    }
    if value.ends_with('Z') {
        let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ").ok()?;
        return Some(naive.and_utc().timestamp_millis());
    }
    for format in ["%Y%m%dT%H%M%S", "%Y%m%dT%H%M"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, format) {
            if let Some(time) = Local.from_local_datetime(&naive).single() {
                return Some(time.timestamp_millis());
            }
        }
    }
    None
}

fn unescape_ics(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

fn escape_ics(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

#[tauri::command]
pub async fn calendar_import_ics(
    state: State<'_, AppState>,
    path: String,
) -> Result<usize, String> {
    let raw = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;

    // ICS folds long lines with a leading space or tab.
    let mut unfolded = String::with_capacity(raw.len());
    for line in raw.replace("\r\n", "\n").split('\n') {
        if line.starts_with(' ') || line.starts_with('\t') {
            unfolded.push_str(&line[1..]);
        } else {
            if !unfolded.is_empty() {
                unfolded.push('\n');
            }
            unfolded.push_str(line);
        }
    }

    let mut events: Vec<(String, i64, i64, String)> = Vec::new();
    let mut in_event = false;
    let mut summary = String::new();
    let mut notes = String::new();
    let mut start: Option<i64> = None;
    let mut end: Option<i64> = None;
    let mut all_day = false;

    for line in unfolded.lines() {
        let line = line.trim_end_matches('\r');
        let upper = line.to_ascii_uppercase();
        if upper == "BEGIN:VEVENT" {
            in_event = true;
            summary.clear();
            notes.clear();
            start = None;
            end = None;
            all_day = false;
            continue;
        }
        if upper == "END:VEVENT" {
            if in_event {
                if let Some(begin) = start {
                    let finish = end.unwrap_or(begin + 3_600_000);
                    let title = if summary.trim().is_empty() {
                        "Imported event".to_string()
                    } else {
                        summary.clone()
                    };
                    events.push((title, begin, finish, notes.clone()));
                }
            }
            in_event = false;
            continue;
        }
        if !in_event {
            continue;
        }
        if let Some(value) = line.strip_prefix("SUMMARY:") {
            summary = unescape_ics(value);
        } else if let Some(value) = line.strip_prefix("DESCRIPTION:") {
            notes = unescape_ics(value);
        } else if let Some(value) = line.strip_prefix("DTSTART") {
            let value = value.split(':').next_back().unwrap_or(value);
            start = parse_ics_time(value);
            all_day = !value.contains('T');
        } else if let Some(value) = line.strip_prefix("DTEND") {
            let value = value.split(':').next_back().unwrap_or(value);
            end = parse_ics_time(value);
        }
    }

    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut imported = 0usize;
    for (title, begin, finish, notes) in events {
        guard
            .execute(
                "INSERT INTO events (title, start, end, all_day, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![title, begin, finish, if all_day { 1 } else { 0 }, notes],
            )
            .map_err(|error| error.to_string())?;
        imported += 1;
    }
    Ok(imported)
}

fn default_export_path() -> std::path::PathBuf {
    let documents = directories::UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(|path| path.to_path_buf()))
        .unwrap_or_else(crate::settings::data_dir);
    documents.join("isle-calendar.ics")
}

#[tauri::command]
pub fn calendar_export_ics(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<String, String> {
    let target = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_export_path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare("SELECT id, title, start, end, all_day, notes FROM events ORDER BY start ASC")
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], row_to_event)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    let mut output = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Isle//Calendar//EN\r\nCALSCALE:GREGORIAN\r\n",
    );
    for event in rows {
        let stamp = chrono::Utc
            .timestamp_millis_opt(event.start)
            .single()
            .map(|time| time.format("%Y%m%dT%H%M%SZ").to_string())
            .unwrap_or_else(|| "19700101T000000Z".into());
        let finish = chrono::Utc
            .timestamp_millis_opt(event.end)
            .single()
            .map(|time| time.format("%Y%m%dT%H%M%SZ").to_string())
            .unwrap_or_else(|| stamp.clone());
        output.push_str("BEGIN:VEVENT\r\n");
        output.push_str(&format!("UID:isle-{}@isle.local\r\n", event.id));
        output.push_str(&format!("DTSTAMP:{stamp}\r\n"));
        output.push_str(&format!("DTSTART:{stamp}\r\n"));
        output.push_str(&format!("DTEND:{finish}\r\n"));
        output.push_str(&format!("SUMMARY:{}\r\n", escape_ics(&event.title)));
        if !event.notes.is_empty() {
            output.push_str(&format!("DESCRIPTION:{}\r\n", escape_ics(&event.notes)));
        }
        output.push_str("END:VEVENT\r\n");
    }
    output.push_str("END:VCALENDAR\r\n");

    std::fs::write(&target, output).map_err(|error| error.to_string())?;
    Ok(target.to_string_lossy().to_string())
}

fn take_due(app: &AppHandle) -> Vec<Reminder> {
    let state = app.state::<AppState>();
    let Ok(guard) = state.db.conn.lock() else {
        return Vec::new();
    };
    let now = now_ms();
    let mut stmt = match guard.prepare(
        "SELECT id, title, due, done, notified, created FROM reminders \
         WHERE done = 0 AND notified = 0 AND due <= ?1 ORDER BY due ASC LIMIT 5",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Vec::new(),
    };
    let due = match stmt.query_map(params![now], row_to_reminder) {
        Ok(rows) => rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    for reminder in &due {
        let _ = guard.execute(
            "UPDATE reminders SET notified = 1 WHERE id = ?1",
            params![reminder.id],
        );
    }
    due
}

pub fn spawn_reminder_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(15));
        for reminder in take_due(&app) {
            let _ = app.emit("reminder-due", &reminder);
            let _ = app
                .notification()
                .builder()
                .title("Isle")
                .body(&reminder.title)
                .show();
        }
    });
}

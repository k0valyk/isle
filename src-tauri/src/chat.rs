use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Serialize, Clone)]
pub struct ChatSession {
    pub id: i64,
    pub title: String,
    pub created: i64,
    pub updated: i64,
    pub messages: i64,
}

#[derive(Serialize, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created: i64,
}

#[derive(Serialize)]
pub struct ChatReply {
    pub session_id: i64,
    pub title: String,
    pub content: String,
    pub model: String,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct Completion {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: Option<ErrorDetail>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatSession> {
    Ok(ChatSession {
        id: row.get(0)?,
        title: row.get(1)?,
        created: row.get(2)?,
        updated: row.get(3)?,
        messages: row.get(4)?,
    })
}

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatMessage> {
    Ok(ChatMessage {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        created: row.get(4)?,
    })
}

#[tauri::command]
pub fn chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT s.id, s.title, s.created, s.updated, \
             (SELECT COUNT(*) FROM chat_messages m WHERE m.session_id = s.id) \
             FROM chat_sessions s ORDER BY s.updated DESC LIMIT 100",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map([], row_to_session)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_session_create(state: State<'_, AppState>, title: Option<String>) -> Result<i64, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let title = title.unwrap_or_else(|| "New chat".into());
    guard
        .execute(
            "INSERT INTO chat_sessions (title, created, updated) VALUES (?1, ?2, ?2)",
            params![title, now_ms()],
        )
        .map_err(|error| error.to_string())?;
    Ok(guard.last_insert_rowid())
}

#[tauri::command]
pub fn chat_session_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    guard
        .execute("DELETE FROM chat_messages WHERE session_id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    guard
        .execute("DELETE FROM chat_sessions WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn chat_messages(state: State<'_, AppState>, session_id: i64) -> Result<Vec<ChatMessage>, String> {
    let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
    let mut stmt = guard
        .prepare(
            "SELECT id, session_id, role, content, created FROM chat_messages \
             WHERE session_id = ?1 ORDER BY id ASC LIMIT 400",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![session_id], row_to_message)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn normalize_endpoint(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn root_endpoint(base: &str) -> String {
    base.trim()
        .trim_end_matches('/')
        .trim_end_matches("/chat/completions")
        .trim_end_matches('/')
        .to_string()
}

fn local_endpoint(url: &reqwest::Url) -> bool {
    match url.host_str() {
        Some(host) => {
            host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host == "0.0.0.0"
                || host.ends_with(".local")
        }
        None => false,
    }
}

#[tauri::command]
pub async fn chat_probe(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (base_url, api_key, model) = {
        let guard = state.settings.lock().map_err(|error| error.to_string())?;
        (
            guard.chat_base_url.clone(),
            guard.chat_api_key.clone(),
            guard.chat_model.clone(),
        )
    };
    let root = root_endpoint(&base_url);
    let url = reqwest::Url::parse(&format!("{root}/models"))
        .map_err(|error| format!("Invalid AI endpoint: {error}"))?;
    if api_key.trim().is_empty() && !local_endpoint(&url) {
        return Err("AI API key is not configured".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.get(url);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Connection failed: {error}"))?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", body.chars().take(240).collect::<String>()));
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("Invalid provider response: {error}"))?;
    let models: Vec<String> = value
        .get("data")
        .and_then(|data| data.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(serde_json::json!({
        "ok": true,
        "endpoint": root,
        "model": model,
        "models": models,
    }))
}

#[tauri::command]
pub async fn chat_send(
    state: State<'_, AppState>,
    session_id: i64,
    message: String,
) -> Result<ChatReply, String> {
    let text = message.trim().to_string();
    if text.is_empty() {
        return Err("Message is empty".into());
    }

    let (base_url, api_key, model) = {
        let guard = state
            .settings
            .lock()
            .map_err(|error| error.to_string())?;
        (
            guard.chat_base_url.clone(),
            guard.chat_api_key.clone(),
            guard.chat_model.clone(),
        )
    };

    let endpoint = normalize_endpoint(&base_url);
    let url = reqwest::Url::parse(&endpoint)
        .map_err(|error| format!("Invalid AI endpoint: {error}"))?;
    if api_key.trim().is_empty() && !local_endpoint(&url) {
        return Err("AI API key is not configured".into());
    }

    let history: Vec<ChatMessage> = {
        let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
        let mut stmt = guard
            .prepare(
                "SELECT id, session_id, role, content, created FROM chat_messages \
                 WHERE session_id = ?1 ORDER BY id DESC LIMIT 30",
            )
            .map_err(|error| error.to_string())?;
        let rows = stmt
            .query_map(params![session_id], row_to_message)
            .map_err(|error| error.to_string())?;
        let mut list = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        list.reverse();
        list
    };

    let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": "You are Isle, a concise desktop assistant living in a small Windows island. Answer briefly and use plain text unless the user asks for code."
    })];
    for item in &history {
        messages.push(serde_json::json!({ "role": item.role, "content": item.content }));
    }
    messages.push(serde_json::json!({ "role": "user", "content": text.clone() }));

    // Persist the user's message before the request so it is not lost when the
    // local model is offline or the endpoint returns an error.
    {
        let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
        guard
            .execute(
                "INSERT INTO chat_messages (session_id, role, content, created) VALUES (?1, 'user', ?2, ?3)",
                params![session_id, text.clone(), now_ms()],
            )
            .map_err(|error| error.to_string())?;
        let existing: String = guard
            .query_row(
                "SELECT title FROM chat_sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let title = if existing.trim().is_empty() || existing == "New chat" {
            text.chars().take(48).collect::<String>()
        } else {
            existing
        };
        guard
            .execute(
                "UPDATE chat_sessions SET title = ?1, updated = ?2 WHERE id = ?3",
                params![title, now_ms(), session_id],
            )
            .map_err(|error| error.to_string())?;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;

    let mut request = client.post(url).json(&serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": 0.7,
        "stream": false,
    }));
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("AI request failed: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("AI response failed: {error}"))?;

    if !status.is_success() {
        let detail = serde_json::from_str::<ErrorBody>(&body)
            .ok()
            .and_then(|value| {
                value
                    .error
                    .and_then(|error| error.message)
                    .or(value.message)
            })
            .unwrap_or_else(|| body.chars().take(300).collect());
        return Err(format!("AI HTTP {status}: {detail}"));
    }

    let completion: Completion = serde_json::from_str(&body)
        .map_err(|error| format!("AI response is invalid: {error}"))?;
    let content = completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "AI returned an empty answer".to_string())?;

    let title = {
        let guard = state.db.conn.lock().map_err(|error| error.to_string())?;
        guard
            .execute(
                "INSERT INTO chat_messages (session_id, role, content, created) VALUES (?1, 'assistant', ?2, ?3)",
                params![session_id, content, now_ms()],
            )
            .map_err(|error| error.to_string())?;

        let existing: String = guard
            .query_row(
                "SELECT title FROM chat_sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        guard
            .execute(
                "UPDATE chat_sessions SET title = ?1, updated = ?2 WHERE id = ?3",
                params![existing, now_ms(), session_id],
            )
            .map_err(|error| error.to_string())?;
        existing
    };

    Ok(ChatReply {
        session_id,
        title,
        content,
        model,
    })
}

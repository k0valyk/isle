use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub provider: String,
    pub model: String,
    pub label: String,
    pub sessions: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_tokens: u64,
    pub cost: f64,
    pub context_used: u64,
    pub context_limit: Option<u64>,
    pub context_left: Option<u64>,
    pub last_active: i64,
}

struct ModelMeta {
    limit: Option<u64>,
    label: String,
}

#[derive(serde::Deserialize)]
struct CatalogProvider {
    #[serde(default)]
    models: HashMap<String, CatalogModel>,
}

#[derive(serde::Deserialize)]
struct CatalogModel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    limit: Option<CatalogLimit>,
}

#[derive(serde::Deserialize)]
struct CatalogLimit {
    #[serde(default)]
    context: Option<u64>,
}

fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

fn insert_model(
    meta: &mut HashMap<String, ModelMeta>,
    provider_id: &str,
    model_id: &str,
    model: &serde_json::Value,
) {
    let limit = model
        .pointer("/limit/context")
        .and_then(|item| item.as_u64());
    let label = model
        .get("name")
        .and_then(|item| item.as_str())
        .unwrap_or(model_id)
        .to_string();
    meta.insert(
        format!("{provider_id}/{model_id}"),
        ModelMeta {
            limit,
            label: label.clone(),
        },
    );
    meta.entry(model_id.to_string()).or_insert(ModelMeta { limit, label });
}

fn load_meta() -> HashMap<String, ModelMeta> {
    let mut meta = HashMap::new();
    let Some(home) = home_dir() else {
        return meta;
    };

    // The OpenCode catalog (`models.dev`) carries context limits for every
    // built-in provider, including the DeepSeek models used here.
    let db = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    if let Ok(conn) = Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        if let Ok(raw) = conn.query_row(
            "select value from kv where key = 'models-dev:v2:catalog'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            if let Ok(outer) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(body) = outer.get("body").and_then(|item| item.as_str()) {
                    if let Ok(providers) =
                        serde_json::from_str::<HashMap<String, CatalogProvider>>(body)
                    {
                        for (provider_id, provider) in providers {
                            for (model_id, model) in provider.models {
                                let limit = model.limit.and_then(|limit| limit.context);
                                let label = if model.name.is_empty() {
                                    model_id.clone()
                                } else {
                                    model.name
                                };
                                meta.insert(
                                    format!("{provider_id}/{model_id}"),
                                    ModelMeta {
                                        limit,
                                        label: label.clone(),
                                    },
                                );
                                meta.entry(model_id).or_insert(ModelMeta { limit, label });
                            }
                        }
                    }
                }
            }
        }
    }

    // Provider overrides in the user's OpenCode config win over the catalog.
    let path = home.join(".config").join("opencode").join("opencode.json");
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(providers) = value.get("provider").and_then(|item| item.as_object()) {
                for (provider_id, provider) in providers {
                    let Some(models) = provider.get("models").and_then(|item| item.as_object())
                    else {
                        continue;
                    };
                    for (model_id, model) in models {
                        insert_model(&mut meta, provider_id, model_id, model);
                    }
                }
            }
        }
    }
    meta
}

fn model_meta() -> &'static HashMap<String, ModelMeta> {
    static META: OnceLock<HashMap<String, ModelMeta>> = OnceLock::new();
    META.get_or_init(load_meta)
}

fn parse_model(raw: &str) -> (String, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return (String::new(), raw.to_string());
    };
    let provider = value
        .get("providerID")
        .and_then(|item| item.as_str())
        .unwrap_or_default()
        .to_string();
    let model = value
        .get("id")
        .and_then(|item| item.as_str())
        .unwrap_or_default()
        .to_string();
    (provider, model)
}

fn latest_message(conn: &Connection, session_id: &str) -> Option<(u64, u64, u64, u64, f64, String, String)> {
    let mut statement = conn
        .prepare(
            "select data from session_message where session_id = ?1 and type = 'assistant' \
             and data like '%\"tokens\"%' order by time_created desc limit 1",
        )
        .ok()?;
    let data: String = statement.query_row([session_id], |row| row.get(0)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&data).ok()?;
    let tokens = value.get("tokens");
    let read = |key: &str| {
        tokens
            .and_then(|item| item.get(key))
            .and_then(|item| item.as_u64())
            .unwrap_or(0)
    };
    let input = read("input");
    let output = read("output");
    let reasoning = read("reasoning");
    let cache = tokens
        .and_then(|item| item.pointer("/cache/read"))
        .and_then(|item| item.as_u64())
        .unwrap_or(0);
    let cost = value
        .get("cost")
        .and_then(|item| item.as_f64())
        .unwrap_or(0.0);
    let (provider, model) = value
        .get("model")
        .and_then(|item| serde_json::to_string(item).ok())
        .map(|raw| parse_model(&raw))
        .unwrap_or_default();
    Some((input, output, reasoning, cache, cost, provider, model))
}

#[tauri::command]
pub fn agent_usage() -> Vec<ModelUsage> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    let db = home
        .join(".local")
        .join("share")
        .join("opencode")
        .join("opencode.db");
    if !db.exists() {
        return Vec::new();
    }
    let Ok(conn) = Connection::open_with_flags(
        &db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return Vec::new();
    };
    let _ = conn.busy_timeout(std::time::Duration::from_millis(120));
    let meta = model_meta();
    let cutoff = chrono::Local::now().timestamp_millis() - 30 * 60 * 1000;

    let mut statement = match conn.prepare(
        "select id, model, cost, tokens_input, tokens_output, tokens_reasoning, \
         tokens_cache_read, time_updated from session_v2 where time_updated > ?1 \
         order by time_updated desc limit 40",
    ) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map([cutoff], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
        ))
    });

    let mut grouped: HashMap<String, ModelUsage> = HashMap::new();
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (session_id, model_raw, session_cost, input, output, reasoning, cache, updated) = row;
            let (provider, model) = parse_model(&model_raw);
            let message = latest_message(&conn, &session_id);
            let (msg_provider, msg_model) = message
                .as_ref()
                .map(|item| (item.5.clone(), item.6.clone()))
                .unwrap_or_default();
            let provider = if msg_provider.is_empty() {
                provider
            } else {
                msg_provider
            };
            let model = if msg_model.is_empty() { model } else { msg_model };
            if model.is_empty() {
                continue;
            }
            let key = format!("{provider}/{model}");
            let entry = grouped.entry(key.clone()).or_insert_with(|| {
                let info = meta.get(&key);
                ModelUsage {
                    provider: provider.clone(),
                    model: model.clone(),
                    label: info.map(|item| item.label.clone()).unwrap_or_else(|| model.clone()),
                    context_limit: info.and_then(|item| item.limit),
                    ..ModelUsage::default()
                }
            });
            entry.sessions += 1;
            entry.input_tokens += input.max(0) as u64;
            entry.output_tokens += output.max(0) as u64;
            entry.reasoning_tokens += reasoning.max(0) as u64;
            entry.cache_tokens += cache.max(0) as u64;
            entry.cost += session_cost;
            entry.last_active = entry.last_active.max(updated);
            if let Some((msg_input, msg_output, msg_reasoning, msg_cache, _, _, _)) = message {
                let used = msg_input + msg_cache + msg_output + msg_reasoning;
                if used > entry.context_used {
                    entry.context_used = used;
                }
            }
            if let Some(limit) = entry.context_limit {
                if entry.context_used > 0 {
                    entry.context_left = Some(limit.saturating_sub(entry.context_used));
                }
            }
        }
    }

    let mut list: Vec<ModelUsage> = grouped.into_values().collect();
    list.sort_by_key(|item| std::cmp::Reverse(item.last_active));
    list.truncate(8);
    list
}

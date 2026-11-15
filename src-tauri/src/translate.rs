use serde::{Deserialize, Serialize};

const MAX_TEXT: usize = 6000;
const CHUNK: usize = 420;

#[derive(Serialize)]
pub struct TranslationResult {
    pub text: String,
    pub from: String,
    pub to: String,
    pub detected: bool,
}

#[derive(Deserialize)]
struct MyMemoryResponse {
    #[serde(default)]
    #[serde(rename = "responseData")]
    response_data: Option<MyMemoryData>,
    #[serde(default)]
    #[serde(rename = "responseStatus")]
    response_status: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct MyMemoryData {
    #[serde(default)]
    #[serde(rename = "translatedText")]
    translated_text: String,
}

fn detect(text: &str) -> String {
    let cyrillic = text.chars().filter(|c| ('\u{0400}'..='\u{04FF}').contains(c)).count();
    let latin = text.chars().filter(|c| c.is_ascii_alphabetic()).count();
    if cyrillic > latin {
        let lower = text.to_lowercase();
        if lower.contains('і')
            || lower.contains('ї')
            || lower.contains('є')
            || lower.contains('ґ')
        {
            "uk".into()
        } else if lower.contains('ы') || lower.contains('э') || lower.contains('ъ') {
            "ru".into()
        } else {
            "uk".into()
        }
    } else {
        "en".into()
    }
}

fn normalize_code(code: &str) -> Option<String> {
    let code = code.trim().to_lowercase();
    if code == "auto" {
        return Some("auto".into());
    }
    if code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(code);
    }
    match code.as_str() {
        "ukrainian" | "uk-ua" | "ua" => Some("uk".into()),
        "english" | "en-us" | "en-gb" => Some("en".into()),
        "russian" | "ru-ru" => Some("ru".into()),
        "polish" | "pl-pl" => Some("pl".into()),
        "german" | "de-de" => Some("de".into()),
        _ => None,
    }
}

fn split_chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for piece in text.split_inclusive('\n') {
        if current.len() + piece.len() > CHUNK && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        if piece.len() > CHUNK {
            let mut rest = piece;
            while rest.len() > CHUNK {
                let mut cut = CHUNK;
                while cut > 0 && !rest.is_char_boundary(cut) {
                    cut -= 1;
                }
                chunks.push(rest[..cut].to_string());
                rest = &rest[cut..];
            }
            current.push_str(rest);
        } else {
            current.push_str(piece);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() {
        chunks.push(String::new());
    }
    chunks
}

fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

async fn translate_google_chunk(
    client: &reqwest::Client,
    text: &str,
    from: &str,
    to: &str,
) -> Result<String, String> {
    let response = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", from),
            ("tl", to),
            ("dt", "t"),
            ("q", text),
        ])
        .send()
        .await
        .map_err(|error| format!("Google translation request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Google translation returned HTTP {}", response.status()));
    }
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("Google translation response is invalid: {error}"))?;
    let mut translated = String::new();
    if let Some(segments) = payload.get(0).and_then(|value| value.as_array()) {
        for segment in segments {
            if let Some(part) = segment.get(0).and_then(|value| value.as_str()) {
                translated.push_str(part);
            }
        }
    }
    if translated.trim().is_empty() {
        return Err("Google translation returned an empty result".into());
    }
    Ok(translated)
}

async fn translate_chunk(
    client: &reqwest::Client,
    text: &str,
    from: &str,
    to: &str,
) -> Result<String, String> {
    match translate_mymemory_chunk(client, text, from, to).await {
        Ok(value) => Ok(value),
        Err(first) => translate_google_chunk(client, text, from, to)
            .await
            .map_err(|second| format!("{first}; {second}")),
    }
}

async fn translate_mymemory_chunk(
    client: &reqwest::Client,
    text: &str,
    from: &str,
    to: &str,
) -> Result<String, String> {
    let pair = format!("{from}|{to}");
    let response = client
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text), ("langpair", pair.as_str())])
        .send()
        .await
        .map_err(|error| format!("Translation request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Translation service returned HTTP {}", response.status()));
    }
    let payload: MyMemoryResponse = response
        .json()
        .await
        .map_err(|error| format!("Translation response is invalid: {error}"))?;
    if let Some(status) = payload.response_status {
        if status.as_i64().unwrap_or(200) != 200 {
            return Err("Translation service rejected the request (daily limit?)".into());
        }
    }
    let translated = payload
        .response_data
        .map(|data| data.translated_text)
        .unwrap_or_default();
    if translated.trim().is_empty() {
        return Err("Translation service returned an empty result".into());
    }
    Ok(unescape(&translated))
}

#[tauri::command]
pub async fn translate_text(
    text: String,
    from: String,
    to: String,
) -> Result<TranslationResult, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Nothing to translate".into());
    }
    if text.len() > MAX_TEXT {
        return Err(format!("Text is longer than {MAX_TEXT} characters"));
    }
    let requested_from = normalize_code(&from).ok_or_else(|| "Unsupported source language".to_string())?;
    let target = normalize_code(&to).ok_or_else(|| "Unsupported target language".to_string())?;
    if target == "auto" {
        return Err("Choose a target language".into());
    }
    let detected = requested_from == "auto";
    let source = if detected { detect(text) } else { requested_from };
    if source == target {
        return Ok(TranslationResult {
            text: text.to_string(),
            from: source,
            to: target,
            detected,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent("Isle/1.6 (Windows desktop island; text translation)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?;

    let mut parts = Vec::new();
    for chunk in split_chunks(text) {
        if chunk.trim().is_empty() {
            parts.push(chunk);
            continue;
        }
        parts.push(translate_chunk(&client, &chunk, &source, &target).await?);
    }
    Ok(TranslationResult {
        text: parts.join(""),
        from: source,
        to: target,
        detected,
    })
}

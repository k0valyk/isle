use serde::Deserialize;
use serde_json::{json, Value};

const LRCLIB: &str = "https://lrclib.net/api";

#[derive(Deserialize)]
struct LyricsRecord {
    #[serde(default)]
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(default)]
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    #[serde(default)]
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(default)]
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(default)]
    instrumental: Option<bool>,
    #[serde(default)]
    duration: Option<f64>,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Isle/0.1.0 (Windows island; local desktop app)")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn record_json(record: LyricsRecord) -> Value {
    json!({
        "track": record.track_name.unwrap_or_default(),
        "artist": record.artist_name.unwrap_or_default(),
        "synced": record.synced_lyrics,
        "plain": record.plain_lyrics,
        "instrumental": record.instrumental.unwrap_or(false),
        "duration": record.duration,
    })
}

fn strip_title_noise(value: &str) -> String {
    let mut text = value.trim().to_string();
    for marker in [
        "(Official Video)",
        "(Official Music Video)",
        "(Official Audio)",
        "(Lyric Video)",
        "(Lyrics)",
        "(Audio)",
        "[Official Video]",
        "[Official Audio]",
        "[Lyrics]",
        "(Visualizer)",
        "(HD)",
        "(HQ)",
    ] {
        text = text.replace(marker, "");
        text = text.replace(&marker.to_lowercase(), "");
    }
    while text.ends_with(" - ") || text.ends_with('-') {
        text.pop();
    }
    text.trim().to_string()
}

fn normalize_metadata(title: &str, artist: &str) -> (String, String) {
    let mut title = strip_title_noise(title);
    let mut artist = artist.trim().to_string();
    if artist.ends_with(" - Topic") {
        artist = artist
            .trim_end_matches(" - Topic")
            .trim()
            .to_string();
    }
    if artist.is_empty() {
        if let Some((left, right)) = title.split_once(" - ") {
            if !left.trim().is_empty() && !right.trim().is_empty() {
                artist = left.trim().to_string();
                title = right.trim().to_string();
            }
        }
    }
    if artist.is_empty() {
        if let Some((left, right)) = title.split_once('–') {
            if !left.trim().is_empty() && !right.trim().is_empty() {
                artist = left.trim().to_string();
                title = right.trim().to_string();
            }
        }
    }
    (strip_title_noise(&title), artist)
}

#[tauri::command]
pub async fn fetch_lyrics(
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<f64>,
) -> Result<Value, String> {
    let (title, artist) = normalize_metadata(&title, &artist);
    if title.is_empty() {
        return Err("Track title is empty".into());
    }

    let http = client();

    // Exact lookup first: it is the most accurate when the player exposes
    // album and duration metadata.
    if !artist.is_empty() {
        let mut query: Vec<(&str, String)> = vec![
            ("track_name", title.clone()),
            ("artist_name", artist.clone()),
        ];
        if let Some(album) = album.as_ref().filter(|value| !value.trim().is_empty()) {
            query.push(("album_name", album.clone()));
        }
        if let Some(duration) = duration.filter(|value| *value > 0.0) {
            query.push(("duration", format!("{duration:.0}")));
        }

        let exact = http
            .get(format!("{LRCLIB}/get"))
            .query(&query)
            .send()
            .await
            .map_err(|error| format!("LRCLIB request failed: {error}"))?;

        if exact.status().is_success() {
            if let Ok(record) = exact.json::<LyricsRecord>().await {
                return Ok(record_json(record));
            }
        }
    }

    let query_text = if artist.is_empty() {
        title.clone()
    } else {
        format!("{artist} {title}")
    };
    let search = http
        .get(format!("{LRCLIB}/search"))
        .query(&[("q", query_text.as_str())])
        .send()
        .await
        .map_err(|error| format!("LRCLIB search failed: {error}"))?;

    if !search.status().is_success() {
        return Err(format!("LRCLIB HTTP {}", search.status()));
    }

    let mut records: Vec<LyricsRecord> = search
        .json()
        .await
        .map_err(|error| format!("LRCLIB response is invalid: {error}"))?;

    if records.is_empty() {
        return Ok(json!({ "synced": null, "plain": null, "instrumental": false }));
    }

    // Prefer records that actually contain lyrics, then those whose artist and
    // title match, then synced lyrics, then the duration closest to playback.
    records.sort_by(|a, b| {
        let score = |record: &LyricsRecord| {
            let mut score = 0i32;
            if let Some(name) = &record.artist_name {
                if !artist.is_empty() && name.to_lowercase().contains(&artist.to_lowercase()) {
                    score += 8;
                }
            }
            if let Some(name) = &record.track_name {
                if name.to_lowercase().contains(&title.to_lowercase()) {
                    score += 6;
                }
            }
            if record.synced_lyrics.is_some() {
                score += 4;
            }
            if record.plain_lyrics.is_some() {
                score += 2;
            }
            if record.instrumental.unwrap_or(false) {
                score += 1;
            }
            if let (Some(target), Some(candidate)) = (duration, record.duration) {
                let delta = (target - candidate).abs();
                if delta < 3.0 {
                    score += 4;
                } else if delta < 12.0 {
                    score += 2;
                }
            }
            score
        };
        score(b).cmp(&score(a))
    });

    Ok(record_json(records.remove(0)))
}

use base64::Engine as _;
use serde::Deserialize;

const MAX_ARTWORK: usize = 6 * 1024 * 1024;

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Isle/1.6 (Windows desktop island; cover art lookup)")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[derive(Deserialize)]
struct ItunesResponse {
    #[serde(default)]
    results: Vec<ItunesResult>,
}

#[derive(Deserialize)]
struct ItunesResult {
    #[serde(default)]
    #[serde(rename = "trackName")]
    track_name: String,
    #[serde(default)]
    #[serde(rename = "artistName")]
    artist_name: String,
    #[serde(default)]
    #[serde(rename = "artworkUrl100")]
    artwork: String,
}

#[derive(Deserialize)]
struct DeezerResponse {
    #[serde(default)]
    data: Vec<DeezerTrack>,
}

#[derive(Deserialize)]
struct DeezerTrack {
    #[serde(default)]
    title: String,
    #[serde(default)]
    artist: DeezerArtist,
    #[serde(default)]
    album: DeezerAlbum,
}

#[derive(Deserialize, Default)]
struct DeezerArtist {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct DeezerAlbum {
    #[serde(default)]
    cover_xl: String,
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .replace(['(', ')', '[', ']', '.', ',', '-', '_', '/'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn similarity(a: &str, b: &str) -> usize {
    let left = normalize(a);
    let right = normalize(b);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let mut score = 0;
    for word in left.split_whitespace() {
        if word.len() > 1 && right.contains(word) {
            score += word.len();
        }
    }
    score
}

async fn download(url: &str) -> Option<String> {
    if !url.starts_with("https://") {
        return None;
    }
    let response = client().get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_ARTWORK {
        return None;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{content_type};base64,{encoded}"))
}

async fn itunes(title: &str, artist: &str) -> Option<String> {
    let term = format!("{artist} {title}");
    let response = client()
        .get("https://itunes.apple.com/search")
        .query(&[("term", term.as_str()), ("entity", "song"), ("limit", "8")])
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload: ItunesResponse = response.json().await.ok()?;
    let mut results = payload.results;
    results.sort_by_key(|item| {
        std::cmp::Reverse(similarity(title, &item.track_name) + similarity(artist, &item.artist_name))
    });
    for item in results {
        if item.artwork.is_empty() {
            continue;
        }
        let url = item
            .artwork
            .replace("100x100bb", "400x400bb")
            .replace("100x100", "400x400");
        if let Some(cover) = download(&url).await {
            return Some(cover);
        }
    }
    None
}

async fn deezer(title: &str, artist: &str) -> Option<String> {
    let query = format!("{artist} {title}");
    let response = client()
        .get("https://api.deezer.com/search")
        .query(&[("q", query.as_str()), ("limit", "5")])
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload: DeezerResponse = response.json().await.ok()?;
    let mut tracks = payload.data;
    tracks.sort_by_key(|item| {
        std::cmp::Reverse(similarity(title, &item.title) + similarity(artist, &item.artist.name))
    });
    for item in tracks {
        if item.album.cover_xl.is_empty() {
            continue;
        }
        if let Some(cover) = download(&item.album.cover_xl).await {
            return Some(cover);
        }
    }
    None
}

/// Fetch a fallback album cover for players whose SMTC session does not expose
/// a thumbnail (some browsers and web players). Only the track metadata leaves
/// the machine; the image is returned as a data URI and never cached to disk.
#[tauri::command]
pub async fn fetch_cover(
    title: String,
    artist: String,
    album: Option<String>,
) -> Result<Option<String>, String> {
    let title = title.trim().to_string();
    let artist = artist.trim().to_string();
    if title.is_empty() {
        return Ok(None);
    }
    let mut cover = itunes(&title, &artist).await;
    if cover.is_none() {
        cover = deezer(&title, &artist).await;
    }
    let _ = album;
    Ok(cover)
}

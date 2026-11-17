use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine as _;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    TogglePlayPause,
    Next,
    Previous,
    Seek(i64),
}

pub struct MediaHandle(pub Mutex<Option<Sender<MediaCommand>>>);

pub struct MediaState {
    latest: Mutex<MediaSnapshot>,
}

#[derive(Serialize, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub active: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub status: String,
    pub position: f64,
    pub duration: f64,
    pub artwork: Option<String>,
    pub source_app: Option<String>,
}

impl MediaSnapshot {
    fn inactive() -> Self {
        Self {
            status: "closed".into(),
            ..Self::default()
        }
    }
}

fn init_winrt() {
    // RoInitialize fails with S_FALSE when the thread is already initialized;
    // both outcomes are safe to ignore.
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
}

fn read_artwork(reference: &IRandomAccessStreamReference) -> Option<String> {
    let stream = reference.OpenReadAsync().ok()?.join().ok()?;
    let content_type = stream
        .ContentType()
        .map(|value| value.to_string())
        .unwrap_or_else(|_| "image/png".into());
    let mime = content_type
        .split([',', ';'])
        .next()
        .unwrap_or("image/jpeg")
        .trim();
    let mime = if mime.starts_with("image/") {
        mime.to_string()
    } else {
        "image/jpeg".to_string()
    };
    let size = stream.Size().ok()?;
    if size == 0 || size > 8 * 1024 * 1024 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut buffer = vec![0u8; size as usize];
    reader.ReadBytes(&mut buffer).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
    Some(format!("data:{mime};base64,{encoded}"))
}

fn status_name(status: GlobalSystemMediaTransportControlsSessionPlaybackStatus) -> &'static str {
    if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
        "playing"
    } else if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused {
        "paused"
    } else if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Stopped {
        "stopped"
    } else if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing {
        "changing"
    } else {
        "closed"
    }
}

fn read_snapshot(
    session: &GlobalSystemMediaTransportControlsSession,
    cached_artwork: &mut Option<String>,
    artwork_key: &mut String,
) -> MediaSnapshot {
    let mut snapshot = MediaSnapshot {
        active: true,
        status: "closed".into(),
        ..MediaSnapshot::default()
    };

    if let Ok(playback) = session.GetPlaybackInfo() {
        if let Ok(status) = playback.PlaybackStatus() {
            snapshot.status = status_name(status).into();
        }
    }

    let mut current_key = String::new();
    if let Ok(operation) = session.TryGetMediaPropertiesAsync() {
        if let Ok(properties) = operation.join() {
            snapshot.title = properties
                .Title()
                .map(|value| value.to_string())
                .unwrap_or_default();
            snapshot.artist = properties
                .Artist()
                .map(|value| value.to_string())
                .unwrap_or_default();
            snapshot.album = properties
                .AlbumTitle()
                .map(|value| value.to_string())
                .unwrap_or_default();
            if snapshot.artist.trim().is_empty() {
                snapshot.artist = properties
                    .AlbumArtist()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
            }
            if let Ok(thumbnail) = properties.Thumbnail() {
                current_key = format!(
                    "{}|{}|{}",
                    snapshot.title, snapshot.artist, snapshot.album
                );
                if current_key != *artwork_key {
                    *cached_artwork = read_artwork(&thumbnail);
                    *artwork_key = current_key.clone();
                }
            }
        }
    }

    if current_key.is_empty() {
        *cached_artwork = None;
        artwork_key.clear();
    }
    snapshot.artwork = cached_artwork.clone();

    if let Ok(timeline) = session.GetTimelineProperties() {
        let start = timeline.StartTime().map(|value| value.Duration).unwrap_or(0);
        let end = timeline.EndTime().map(|value| value.Duration).unwrap_or(0);
        let position = timeline
            .Position()
            .map(|value| value.Duration)
            .unwrap_or(0);
        snapshot.position = ((position - start) as f64 / 10_000_000.0).max(0.0);
        snapshot.duration = ((end - start) as f64 / 10_000_000.0).max(0.0);
        if snapshot.duration > 0.0 {
            snapshot.position = snapshot.position.min(snapshot.duration);
        }
    }
    snapshot.position = (snapshot.position * 10.0).round() / 10.0;

    if snapshot.title.trim().is_empty() {
        // Sessions without metadata generally belong to games or short video
        // clips. Keep them visible but neutral rather than inventing a title.
        snapshot.title = String::new();
    }

    snapshot
}

fn session_playing(session: &GlobalSystemMediaTransportControlsSession) -> bool {
    session
        .GetPlaybackInfo()
        .ok()
        .and_then(|info| info.PlaybackStatus().ok())
        .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false)
}

/// Prefer the current session when it is playing, otherwise pick any session
/// that is playing. This is what makes YouTube in a browser, YouTube Music,
/// Spotify and desktop players all work without choosing them manually.
fn select_session(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
) -> Option<GlobalSystemMediaTransportControlsSession> {
    let current = manager.GetCurrentSession().ok();
    if current.as_ref().map(session_playing).unwrap_or(false) {
        return current;
    }

    if let Ok(sessions) = manager.GetSessions() {
        if let Ok(size) = sessions.Size() {
            for index in 0..size {
                if let Ok(session) = sessions.GetAt(index) {
                    if session_playing(&session) {
                        return Some(session);
                    }
                }
            }
        }
    }

    current.or_else(|| {
        manager
            .GetSessions()
            .ok()
            .and_then(|sessions| sessions.GetAt(0).ok())
    })
}

fn apply_command(session: &GlobalSystemMediaTransportControlsSession, command: MediaCommand) {    let _ = match command {
        MediaCommand::TogglePlayPause => session
            .TryTogglePlayPauseAsync()
            .and_then(|operation| operation.join()),
        MediaCommand::Next => session
            .TrySkipNextAsync()
            .and_then(|operation| operation.join()),
        MediaCommand::Previous => session
            .TrySkipPreviousAsync()
            .and_then(|operation| operation.join()),
        MediaCommand::Seek(seconds) => session
            .TryChangePlaybackPositionAsync(seconds.clamp(0, 24 * 3600) * 10_000_000)
            .and_then(|operation| operation.join()),
    };
}

fn run(app: AppHandle, commands: Receiver<MediaCommand>) {
    init_winrt();

    let manager = loop {
        match GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .and_then(|operation| operation.join())
        {
            Ok(manager) => break manager,
            Err(error) => {
                eprintln!("Isle: media session manager unavailable: {error}");
                std::thread::sleep(Duration::from_secs(4));
            }
        }
    };

    let mut last_payload = String::new();
    let mut emitted_artwork_key = String::new();
    let mut cached_artwork: Option<String> = None;
    let mut artwork_key = String::new();

    loop {
        let current = select_session(&manager);

        while let Ok(command) = commands.try_recv() {
            if let Some(session) = current.as_ref() {
                apply_command(session, command);
            }
        }

        let snapshot = match current.as_ref() {
            Some(session) => {
                let mut snapshot =
                    read_snapshot(session, &mut cached_artwork, &mut artwork_key);
                snapshot.source_app = session
                    .SourceAppUserModelId()
                    .map(|value| value.to_string())
                    .ok();
                snapshot
            }
            None => {
                cached_artwork = None;
                artwork_key.clear();
                emitted_artwork_key.clear();
                MediaSnapshot::inactive()
            }
        };

        // The frontend interpolates the playhead between events, so there is no
        // need to push the whole snapshot (including cover art) every 400 ms.
        let artwork_changed = !artwork_key.is_empty() && artwork_key != emitted_artwork_key;
        if artwork_changed {
            emitted_artwork_key = artwork_key.clone();
        }
        let mut payload = snapshot.clone();
        if !artwork_changed && payload.active {
            payload.artwork = None;
        }

        if let Ok(mut latest) = app.state::<MediaState>().latest.lock() {
            *latest = snapshot.clone();
        }

        let comparison = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            payload.active,
            payload.title,
            payload.artist,
            payload.album,
            payload.status,
            (payload.position / 2.0).round() as i64,
            payload.duration.round() as i64,
            emitted_artwork_key
        );
        if comparison != last_payload {
            let _ = app.emit("media-update", &payload);
            last_payload = comparison;
        }

        std::thread::sleep(Duration::from_millis(400));
    }
}

#[tauri::command]
pub fn media_current(state: tauri::State<'_, MediaState>) -> MediaSnapshot {
    state
        .latest
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

pub fn init(app: &AppHandle) {
    let (sender, receiver) = channel();
    app.manage(MediaHandle(Mutex::new(Some(sender))));
    app.manage(MediaState {
        latest: Mutex::new(MediaSnapshot::inactive()),
    });
    let handle = app.clone();
    std::thread::spawn(move || run(handle, receiver));
}

pub fn send(app: &AppHandle, command: MediaCommand) {
    if let Some(state) = app.try_state::<MediaHandle>() {
        if let Ok(guard) = state.0.lock() {
            if let Some(sender) = guard.as_ref() {
                let _ = sender.send(command);
            }
        }
    }
}

#[tauri::command]
pub fn media_control(state: tauri::State<'_, MediaHandle>, action: String, seconds: Option<i64>) {
    let command = match action.as_str() {
        "play_pause" | "toggle" => MediaCommand::TogglePlayPause,
        "next" => MediaCommand::Next,
        "previous" | "prev" => MediaCommand::Previous,
        "seek" => MediaCommand::Seek(seconds.unwrap_or(0)),
        _ => return,
    };
    if let Ok(guard) = state.0.lock() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.send(command);
        }
    }
}

//! macOS media bridge.
//!
//! The Windows build reads the system media session through SMTC. macOS exposes
//! the same information through MediaRemote (private) or per-app scripting; that
//! bridge is not implemented yet, so the island simply reports "nothing
//! playing" instead of failing to build.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[allow(dead_code)]
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

#[tauri::command]
pub fn media_current(state: State<'_, MediaState>) -> MediaSnapshot {
    state
        .latest
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn media_control(state: State<'_, MediaHandle>, action: String) -> Result<(), String> {
    let _ = state;
    Err(format!("Media control '{action}' is not wired up on macOS yet"))
}

pub fn init(app: &AppHandle) {
    let (sender, _receiver) = std::sync::mpsc::channel();
    app.manage(MediaHandle(Mutex::new(Some(sender))));
    app.manage(MediaState {
        latest: Mutex::new(MediaSnapshot {
            status: "closed".into(),
            ..MediaSnapshot::default()
        }),
    });
}

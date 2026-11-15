//! macOS spectrum bridge.
//!
//! WASAPI loopback does not exist on macOS. Capturing the output requires a
//! CoreAudio process tap (14.4+) or a virtual device, so for now the island
//! reports a flat spectrum while still showing the CSS visualisers.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Clone, Default)]
pub struct Spectrum {
    pub bands: Vec<f32>,
    pub level: f32,
}

#[derive(Default)]
pub struct AudioState {
    latest: Mutex<Spectrum>,
}

#[tauri::command]
pub fn audio_spectrum(state: State<'_, AudioState>) -> Spectrum {
    state
        .latest
        .lock()
        .map(|spectrum| spectrum.clone())
        .unwrap_or_default()
}

pub fn init(app: AppHandle) {
    app.manage(AudioState::default());
}

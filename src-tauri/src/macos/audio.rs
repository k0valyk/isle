//! macOS spectrum bridge.
//!
//! WASAPI loopback does not exist on macOS. Capturing the output requires a
//! CoreAudio process tap (14.4+) or a virtual device, so for now the island
//! reports a flat spectrum while still showing the CSS visualisers.

use std::process::Command;
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

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct VolumeState {
    pub level: f32,
    pub muted: bool,
}

fn osascript(script: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("osascript failed".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_volume() -> Result<VolumeState, String> {
    let level: f32 = osascript("output volume of (get volume settings)")?
        .parse()
        .map_err(|_| "could not read the output volume".to_string())?;
    let muted = osascript("output muted of (get volume settings)")? == "true";
    Ok(VolumeState {
        level: (level / 100.0).clamp(0.0, 1.0),
        muted,
    })
}

#[tauri::command]
pub fn system_volume() -> VolumeState {
    read_volume().unwrap_or_default()
}

#[tauri::command]
pub fn set_system_volume(level: f32) -> Result<VolumeState, String> {
    let percent = (level.clamp(0.0, 1.0) * 100.0).round() as i32;
    osascript(&format!("set volume output volume {percent}"))?;
    if percent > 0 && read_volume().map(|volume| volume.muted).unwrap_or(false) {
        osascript("set volume output muted false")?;
    }
    read_volume()
}

#[tauri::command]
pub fn set_system_mute(muted: bool) -> Result<VolumeState, String> {
    osascript(&format!(
        "set volume output muted {}",
        if muted { "true" } else { "false" }
    ))?;
    read_volume()
}

pub fn init(app: AppHandle) {
    app.manage(AudioState::default());
}

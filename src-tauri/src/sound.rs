use std::f32::consts::PI;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

const SAMPLE_RATE: u32 = 44_100;
const SND_FILENAME: u32 = 0x0002_0000;
const SND_ASYNC: u32 = 0x0001;
const SND_NODEFAULT: u32 = 0x0002;
const SND_STOP: u32 = 0x0004;

#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(sound: *const u16, module: isize, flags: u32) -> i32;
}

#[derive(Default)]
pub struct TimerState {
    generation: AtomicU64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TimerFinished {
    reason: String,
}

fn write_wav(path: &std::path::Path) -> Result<(), String> {
    let total_ms = 2_700u32;
    let samples = (SAMPLE_RATE as u64 * total_ms as u64 / 1000) as usize;
    let notes: [(f32, f32, f32); 4] = [
        (659.25, 0.00, 0.50),
        (783.99, 0.38, 0.46),
        (1046.50, 0.76, 0.42),
        (1318.51, 1.46, 0.34),
    ];
    let mut buffer = Vec::with_capacity(samples);
    for index in 0..samples {
        let time = index as f32 / SAMPLE_RATE as f32;
        let mut value = 0.0f32;
        for (frequency, start, amplitude) in notes {
            let local = time - start;
            if local < 0.0 || local > 1.05 {
                continue;
            }
            let attack = (local / 0.012).min(1.0);
            let decay = (-3.2 * local).exp();
            let envelope = attack * decay * amplitude;
            value += (2.0 * PI * frequency * local).sin() * envelope;
            value += (2.0 * PI * frequency * 2.0 * local).sin() * envelope * 0.18;
        }
        let sample = (value.clamp(-1.0, 1.0) * 0.72 * i16::MAX as f32) as i16;
        buffer.extend_from_slice(&sample.to_le_bytes());
    }

    let data_len = buffer.len() as u32;
    let mut file = std::fs::File::create(path).map_err(|error| error.to_string())?;
    file.write_all(b"RIFF").map_err(|error| error.to_string())?;
    file.write_all(&(36 + data_len).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(b"WAVEfmt ").map_err(|error| error.to_string())?;
    file.write_all(&16u32.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&1u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&1u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&SAMPLE_RATE.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(SAMPLE_RATE * 2).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&2u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&16u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(b"data").map_err(|error| error.to_string())?;
    file.write_all(&data_len.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&buffer).map_err(|error| error.to_string())?;
    Ok(())
}

fn sound_path() -> Result<std::path::PathBuf, String> {
    let dir = crate::settings::data_dir().join("sounds");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join("timer.wav");
    if !path.exists() {
        write_wav(&path)?;
    }
    Ok(path)
}

fn play_file(path: &std::path::Path) -> bool {
    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        PlaySoundW(
            wide.as_ptr(),
            0,
            SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
        ) == 1
    }
}

pub fn play() -> Result<(), String> {
    let path = sound_path()?;
    if play_file(&path) {
        Ok(())
    } else {
        Err("Could not play the timer sound".into())
    }
}

pub fn stop() {
    unsafe {
        PlaySoundW(std::ptr::null(), 0, SND_STOP);
    }
}

#[tauri::command]
pub fn play_timer_sound() -> Result<(), String> {
    play()
}

#[tauri::command]
pub fn stop_timer_sound() {
    stop();
}

#[tauri::command]
pub fn timer_schedule(app: AppHandle, seconds: u64) {
    let state = app.state::<TimerState>();
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        let start = Instant::now();
        let duration = Duration::from_secs(seconds);
        while start.elapsed() < duration {
            std::thread::sleep(Duration::from_millis(180));
            if app.state::<TimerState>().generation.load(Ordering::SeqCst) != generation {
                return;
            }
        }
        if app.state::<TimerState>().generation.load(Ordering::SeqCst) != generation {
            return;
        }
        let _ = play();
        let _ = app.emit(
            "timer-finished",
            TimerFinished {
                reason: "timer".into(),
            },
        );
    });
}

#[tauri::command]
pub fn timer_cancel(app: AppHandle) {
    app.state::<TimerState>()
        .generation
        .fetch_add(1, Ordering::SeqCst);
    stop();
}

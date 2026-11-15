use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode};

const FFT_SIZE: usize = 1024;
const BANDS: usize = 12;

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
    std::thread::spawn(move || {
        let hr = wasapi::initialize_mta();
        if hr.is_err() {
            eprintln!("Isle: audio visualizer COM init failed: {hr:?}");
            return;
        }
        loop {
            if let Err(error) = capture_session(&app) {
                eprintln!("Isle: audio visualizer paused: {error}");
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

fn capture_session(app: &AppHandle) -> Result<(), String> {
    let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
    let device = enumerator
        .get_default_device(Direction::Render)
        .map_err(|error| error.to_string())?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|error| error.to_string())?;
    let format = audio_client
        .get_mixformat()
        .map_err(|error| error.to_string())?;

    let channels = format.get_nchannels().max(1) as usize;
    let bits = format.get_bitspersample().max(16);
    let sample_rate = format.get_samplespersec().max(8000);
    let is_float = matches!(
        format.get_subformat().unwrap_or(SampleType::Float),
        SampleType::Float
    );

    let (default_period, _) = audio_client
        .get_device_period()
        .map_err(|error| error.to_string())?;
    let mode = StreamMode::EventsShared {
        autoconvert: false,
        buffer_duration_hns: default_period,
    };
    // Render device + Capture stream = WASAPI loopback of everything the user hears.
    audio_client
        .initialize_client(&format, Direction::Capture, mode, None)
        .map_err(|error| error.to_string())?;
    let event = audio_client
        .set_get_eventhandle()
        .map_err(|error| error.to_string())?;
    let capture = audio_client
        .get_audiocaptureclient()
        .map_err(|error| error.to_string())?;
    let mut queue: VecDeque<u8> = VecDeque::with_capacity(64 * 1024);
    audio_client
        .start_stream()
        .map_err(|error| error.to_string())?;

    let mut planner = FftPlanner::<f32>::new();
    let fft: Arc<dyn Fft<f32>> = planner.plan_fft_forward(FFT_SIZE);
    let mut mono: Vec<f32> = Vec::with_capacity(8192);
    let mut smoothed = vec![0f32; BANDS];
    let mut last_emit = Instant::now() - Duration::from_millis(100);

    loop {
        let read = capture.read_from_device_to_deque(&mut queue);
        if let Err(error) = read {
            let _ = audio_client.stop_stream();
            return Err(error.to_string());
        }

        drain_samples(&mut queue, channels, bits, is_float, &mut mono);

        if last_emit.elapsed() >= Duration::from_millis(33) && mono.len() >= 128 {
            let spectrum = analyze(&mono, sample_rate, fft.as_ref(), &mut smoothed);
            let _ = app.emit("audio-spectrum", &spectrum);
            if let Ok(mut latest) = app.state::<AudioState>().latest.lock() {
                *latest = spectrum.clone();
            }
            last_emit = Instant::now();
        }

        if event.wait_for_event(500_000).is_err() {
            let _ = audio_client.stop_stream();
            return Err("audio capture event timed out".into());
        }
    }
}

fn drain_samples(
    queue: &mut VecDeque<u8>,
    channels: usize,
    bits: u16,
    is_float: bool,
    mono: &mut Vec<f32>,
) {
    let bytes_per_sample = (bits as usize / 8).max(2);
    let frame_bytes = bytes_per_sample * channels;
    while queue.len() >= frame_bytes {
        let mut sum = 0f32;
        for _ in 0..channels {
            let value = match (is_float, bytes_per_sample) {
                (true, 4) => {
                    let mut bytes = [0u8; 4];
                    for byte in &mut bytes {
                        *byte = queue.pop_front().unwrap_or(0);
                    }
                    f32::from_le_bytes(bytes)
                }
                (false, 2) => {
                    let mut bytes = [0u8; 2];
                    for byte in &mut bytes {
                        *byte = queue.pop_front().unwrap_or(0);
                    }
                    i16::from_le_bytes(bytes) as f32 / 32768.0
                }
                (false, 3) => {
                    let b0 = queue.pop_front().unwrap_or(0) as i32;
                    let b1 = queue.pop_front().unwrap_or(0) as i32;
                    let b2 = queue.pop_front().unwrap_or(0) as i32;
                    let raw = b0 | (b1 << 8) | (b2 << 16);
                    let signed = (raw << 8) >> 8;
                    signed as f32 / 8_388_608.0
                }
                (false, 4) => {
                    let mut bytes = [0u8; 4];
                    for byte in &mut bytes {
                        *byte = queue.pop_front().unwrap_or(0);
                    }
                    i32::from_le_bytes(bytes) as f32 / 2_147_483_648.0
                }
                _ => 0.0,
            };
            sum += value.clamp(-1.0, 1.0);
        }
        mono.push(sum / channels as f32);
    }
    if mono.len() > 8192 {
        mono.drain(..mono.len() - 8192);
    }
}

fn analyze(
    mono: &[f32],
    sample_rate: u32,
    fft: &dyn Fft<f32>,
    smoothed: &mut [f32],
) -> Spectrum {
    let start = mono.len().saturating_sub(FFT_SIZE);
    let slice = &mono[start..];
    let mut buffer: Vec<Complex32> = (0..FFT_SIZE)
        .map(|index| {
            let sample = slice.get(index).copied().unwrap_or(0.0);
            let window = 0.5 - 0.5 * (2.0 * PI * index as f32 / (FFT_SIZE as f32 - 1.0)).cos();
            Complex32::new(sample * window, 0.0)
        })
        .collect();
    fft.process(&mut buffer);

    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let min_hz = 45.0f32;
    let max_hz = (sample_rate as f32 / 2.0).min(16_000.0);
    let mut bands = vec![0f32; BANDS];

    for (band, value) in bands.iter_mut().enumerate() {
        let low = min_hz * (max_hz / min_hz).powf(band as f32 / BANDS as f32);
        let high = min_hz * (max_hz / min_hz).powf((band + 1) as f32 / BANDS as f32);
        let first = (low / bin_hz).floor().max(1.0) as usize;
        let last = (high / bin_hz).ceil().min((FFT_SIZE / 2 - 1) as f32) as usize;
        let mut peak = 0f32;
        let end = last.min(FFT_SIZE / 2 - 1);
        if first <= end {
            for value in &buffer[first..=end] {
                peak = peak.max(value.norm());
            }
        }
        let normalized = (peak / (FFT_SIZE as f32 * 0.25)).clamp(0.0, 1.0);
        let shaped = normalized.powf(0.55);
        *value = shaped;
    }

    for index in 0..BANDS {
        smoothed[index] = smoothed[index] * 0.62 + bands[index] * 0.38;
        bands[index] = smoothed[index].clamp(0.0, 1.0);
    }

    let rms = (mono.iter().rev().take(FFT_SIZE).map(|s| s * s).sum::<f32>()
        / FFT_SIZE as f32)
        .sqrt();
    let level = (rms * 6.0).clamp(0.0, 1.0);

    Spectrum { bands, level }
}

//! macOS system statistics through `sysinfo` (CPU/RAM/disk).
//! GPU load and temperatures need IOKit/SMC and are reported as unavailable.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use sysinfo::{Disks, System};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu: f32,
    pub gpu: Option<f32>,
    pub ram: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub disk: f32,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub net_down: f64,
    pub net_up: f64,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub refresh_hz: Option<u32>,
    pub screen_width: u32,
    pub screen_height: u32,
    pub cpu_name: String,
    pub gpu_name: String,
    pub motherboard: String,
    pub ram_modules: String,
    pub os_name: String,
    pub fps: Option<f32>,
    pub fps_process: String,
    pub fps_error: String,
    pub checked_at: i64,
}

pub struct StatsState {
    latest: Mutex<SystemStats>,
}

#[tauri::command]
pub fn system_stats(state: State<'_, StatsState>) -> SystemStats {
    state.latest.lock().map(|stats| stats.clone()).unwrap_or_default()
}

pub fn init(app: AppHandle) {
    app.manage(StatsState {
        latest: Mutex::new(SystemStats::default()),
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut system = System::new();
        let mut disks = Disks::new_with_refreshed_list();
        loop {
            system.refresh_cpu_usage();
            system.refresh_memory();
            disks.refresh(true);

            let total = system.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
            let used = system.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
            let mut disk_total = 0.0f32;
            let mut disk_used = 0.0f32;
            for disk in disks.list() {
                disk_total += disk.total_space() as f32 / 1024.0 / 1024.0 / 1024.0;
                disk_used += (disk.total_space().saturating_sub(disk.available_space())) as f32
                    / 1024.0
                    / 1024.0
                    / 1024.0;
            }

            let stats = SystemStats {
                cpu: system.global_cpu_usage(),
                gpu: None,
                ram: if total > 0.0 { used / total * 100.0 } else { 0.0 },
                ram_used_gb: used,
                ram_total_gb: total,
                disk: if disk_total > 0.0 {
                    disk_used / disk_total * 100.0
                } else {
                    0.0
                },
                disk_used_gb: disk_used,
                disk_total_gb: disk_total,
                net_down: 0.0,
                net_up: 0.0,
                cpu_temp: None,
                gpu_temp: None,
                refresh_hz: None,
                screen_width: 0,
                screen_height: 0,
                cpu_name: system
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().to_string())
                    .unwrap_or_default(),
                gpu_name: String::new(),
                motherboard: String::new(),
                ram_modules: String::new(),
                os_name: [System::name(), System::os_version()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                fps: None,
                fps_process: String::new(),
                fps_error: String::new(),
                checked_at: chrono::Local::now().timestamp_millis(),
            };

            if let Some(state) = handle.try_state::<StatsState>() {
                if let Ok(mut latest) = state.latest.lock() {
                    *latest = stats.clone();
                }
            }
            let _ = tauri::Emitter::emit(&handle, "system-stats", stats);
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

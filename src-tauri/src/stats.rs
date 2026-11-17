use std::os::windows::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use windows::core::PCWSTR;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    fn GetDiskFreeSpaceExW(
        directory: *const u16,
        free_to_caller: *mut u64,
        total: *mut u64,
        total_free: *mut u64,
    ) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn GetDC(window: isize) -> isize;
    fn ReleaseDC(window: isize, dc: isize) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn GetDeviceCaps(dc: isize, index: i32) -> i32;
}

const VREFRESH: i32 = 116;
const HORZRES: i32 = 8;
const VERTRES: i32 = 10;

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

#[derive(Clone, Default)]
struct HardwareInfo {
    cpu_name: String,
    gpu_name: String,
    motherboard: String,
    ram_modules: String,
    os_name: String,
}

pub struct StatsState {
    latest: Mutex<SystemStats>,
    hardware: Mutex<HardwareInfo>,
}

#[tauri::command]
pub fn system_stats(state: State<'_, StatsState>) -> SystemStats {
    state.latest.lock().map(|stats| stats.clone()).unwrap_or_default()
}

fn filetime_value(value: FileTime) -> u64 {
    ((value.high as u64) << 32) | value.low as u64
}

fn read_cpu(previous: &mut Option<(u64, u64)>) -> f32 {
    let mut idle = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return 0.0;
    }
    let idle = filetime_value(idle);
    let busy = filetime_value(kernel) + filetime_value(user);
    let Some((last_idle, last_busy)) = *previous else {
        *previous = Some((idle, busy));
        return 0.0;
    };
    let idle_delta = idle.saturating_sub(last_idle) as f64;
    let busy_delta = busy.saturating_sub(last_busy) as f64;
    *previous = Some((idle, busy));
    if busy_delta <= 0.0 {
        return 0.0;
    }
    (100.0 * (1.0 - idle_delta / busy_delta)).clamp(0.0, 100.0) as f32
}

fn read_ram() -> (f32, f32, f32) {
    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    if ok == 0 || status.total_phys == 0 {
        return (0.0, 0.0, 0.0);
    }
    let used = status.total_phys.saturating_sub(status.avail_phys);
    let percent = (used as f64 / status.total_phys as f64 * 100.0).clamp(0.0, 100.0) as f32;
    (
        percent,
        used as f32 / 1024.0 / 1024.0 / 1024.0,
        status.total_phys as f32 / 1024.0 / 1024.0 / 1024.0,
    )
}

fn read_disk() -> (f32, f32, f32) {
    let root: Vec<u16> = "C:\\\0".encode_utf16().collect();
    let mut free_to_caller = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            root.as_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 || total == 0 {
        return (0.0, 0.0, 0.0);
    }
    let used = total.saturating_sub(total_free);
    let percent = (used as f64 / total as f64 * 100.0).clamp(0.0, 100.0) as f32;
    (
        percent,
        used as f32 / 1024.0 / 1024.0 / 1024.0,
        total as f32 / 1024.0 / 1024.0 / 1024.0,
    )
}

fn read_screen() -> (Option<u32>, u32, u32) {
    unsafe {
        let dc = GetDC(0);
        if dc == 0 {
            return (None, 0, 0);
        }
        let refresh = GetDeviceCaps(dc, VREFRESH);
        let width = GetDeviceCaps(dc, HORZRES);
        let height = GetDeviceCaps(dc, VERTRES);
        ReleaseDC(0, dc);
        (
            (refresh > 0).then_some(refresh as u32),
            width.max(0) as u32,
            height.max(0) as u32,
        )
    }
}

struct PdhArrayCounter {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
}

impl PdhArrayCounter {
    fn new(path: &str) -> Option<Self> {
        let wide: Vec<u16> = format!("{path}\0").encode_utf16().collect();
        unsafe {
            let mut query = PDH_HQUERY::default();
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let mut counter = PDH_HCOUNTER::default();
            if PdhAddEnglishCounterW(query, PCWSTR(wide.as_ptr()), 0, &mut counter) != 0 {
                return None;
            }
            let _ = PdhCollectQueryData(query);
            Some(Self { query, counter })
        }
    }

    fn read(&self) -> Option<f32> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            let mut size = 0u32;
            let mut count = 0u32;
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                None,
            );
            if status != PDH_MORE_DATA || size == 0 {
                return None;
            }
            let mut buffer = vec![0u8; size as usize];
            let ptr = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(ptr),
            );
            if status != 0 || count == 0 {
                return None;
            }
            let items = std::slice::from_raw_parts(ptr, count as usize);
            let mut total = 0f64;
            for item in items {
                let value = item.FmtValue.Anonymous.doubleValue;
                if value.is_finite() && value > 0.0 {
                    total += value;
                }
            }
            Some(total as f32)
        }
    }
}

fn hidden_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn read_hardware() -> HardwareInfo {
    // One WMI round-trip at startup. These values never change while the app
    // runs, so keeping them cached avoids spawning PowerShell every second.
    let script = "$ErrorActionPreference='SilentlyContinue'; \
        $cpu=(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name); \
        $gpu=(Get-CimInstance Win32_VideoController | Where-Object { $_.Name -notmatch 'Microsoft Basic|Remote|Virtual|Parsec' } | Select-Object -First 1 -ExpandProperty Name); \
        $boardItem=Get-CimInstance Win32_BaseBoard | Select-Object -First 1; \
        $board=('{0} {1}' -f $boardItem.Manufacturer, $boardItem.Product).Trim(); \
        $modules=@(Get-CimInstance Win32_PhysicalMemory); \
        $ram=($modules | ForEach-Object { '{0:N0} GB' -f ($_.Capacity/1GB) }) -join ' + '; \
        $speed=($modules | Select-Object -First 1 -ExpandProperty ConfiguredClockSpeed); \
        $os=(Get-CimInstance Win32_OperatingSystem | Select-Object -First 1 -ExpandProperty Caption); \
        [PSCustomObject]@{ cpu=$cpu; gpu=$gpu; board=$board; ram=$ram; speed=$speed; os=$os } | ConvertTo-Json -Compress";
    let output = match hidden_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return HardwareInfo::default(),
    };
    let raw = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = match serde_json::from_str(raw.trim()) {
        Ok(value) => value,
        Err(_) => return HardwareInfo::default(),
    };
    let text = |key: &str| {
        value
            .get(key)
            .and_then(|item| item.as_str())
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let speed = value
        .get("speed")
        .and_then(|item| item.as_u64())
        .filter(|speed| *speed > 0);
    let mut ram = text("ram");
    if let Some(speed) = speed {
        if !ram.is_empty() {
            ram = format!("{ram} · {speed} MHz");
        }
    }
    HardwareInfo {
        cpu_name: text("cpu"),
        gpu_name: text("gpu"),
        motherboard: text("board"),
        ram_modules: ram,
        os_name: text("os"),
    }
}

fn read_gpu_temperature() -> Option<f32> {
    let output = hidden_command("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().parse::<f32>().ok())
        .filter(|value| *value > 0.0 && *value < 130.0)
}

fn read_cpu_temperature() -> Option<f32> {
    // root/WMI exposes the ACPI thermal zone on many laptops. It is a
    // best-effort metric: firmware may refuse the query, and the call needs
    // the sensor to report a tenth-of-a-Kelvin value.
    let script = "$ErrorActionPreference='SilentlyContinue'; \
        $v = Get-CimInstance -Namespace root/WMI -ClassName MSAcpi_ThermalZoneTemperature \
        | Select-Object -First 1 -ExpandProperty CurrentTemperature; \
        if ($v) { [math]::Round(($v / 10) - 273.15, 1) }";
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().replace(',', ".").parse::<f32>().ok())
        .filter(|value| *value > -20.0 && *value < 130.0)
}

fn spawn_temperature_watcher(temperatures: Arc<Mutex<(Option<f32>, Option<f32>)>>, first: bool) {
    std::thread::spawn(move || {
        let mut attempts = 0;
        let mut delay = if first { 4_000 } else { 30_000 };
        loop {
            std::thread::sleep(Duration::from_millis(delay));
            let gpu = read_gpu_temperature();
            let cpu = read_cpu_temperature();
            if let Ok(mut values) = temperatures.lock() {
                *values = (cpu, gpu);
            }
            attempts += 1;
            if cpu.is_none() && gpu.is_none() {
                // Do not keep spawning PowerShell forever on machines whose
                // firmware or permissions do not expose a thermal zone.
                delay = if attempts < 3 { 30_000 } else { 600_000 };
            } else {
                delay = 20_000;
            }
        }
    });
}

pub fn init(app: AppHandle) {
    let temperatures = Arc::new(Mutex::new((None, None)));
    app.manage(StatsState {
        latest: Mutex::new(SystemStats::default()),
        hardware: Mutex::new(HardwareInfo::default()),
    });

    let hardware_app = app.clone();
    std::thread::spawn(move || {
        // Give the first stats sample and the UI priority; WMI can take a
        // moment on cold starts.
        std::thread::sleep(Duration::from_millis(1800));
        let info = read_hardware();
        if let Ok(mut guard) = hardware_app
            .state::<StatsState>()
            .hardware
            .lock()
        {
            *guard = info;
        }
    });

    std::thread::spawn(move || {
        let mut previous = None;
        let gpu = PdhArrayCounter::new("\\GPU Engine(*)\\Utilization Percentage");
        let net_down = PdhArrayCounter::new("\\Network Interface(*)\\Bytes Received/sec");
        let net_up = PdhArrayCounter::new("\\Network Interface(*)\\Bytes Sent/sec");
        let (refresh, screen_width, screen_height) = read_screen();
        // The first PDH sample is a baseline; wait before the next one.
        std::thread::sleep(Duration::from_millis(1000));
        spawn_temperature_watcher(temperatures.clone(), true);
        loop {
            let cpu = read_cpu(&mut previous);
            let (ram, used, total) = read_ram();
            let (disk, disk_used, disk_total) = read_disk();
            let gpu_value = gpu.as_ref().and_then(|counter| counter.read());
            let down = net_down
                .as_ref()
                .and_then(|counter| counter.read())
                .unwrap_or(0.0) as f64;
            let up = net_up
                .as_ref()
                .and_then(|counter| counter.read())
                .unwrap_or(0.0) as f64;
            let (cpu_temp, gpu_temp) = temperatures
                .lock()
                .map(|temps| *temps)
                .unwrap_or((None, None));
            let hardware = app
                .state::<StatsState>()
                .hardware
                .lock()
                .map(|info| info.clone())
                .unwrap_or_default();
            let fps = app
                .try_state::<crate::fps::FpsState>()
                .map(|state| state.get())
                .unwrap_or_default();
            let stats = SystemStats {
                cpu,
                gpu: gpu_value,
                ram,
                ram_used_gb: used,
                ram_total_gb: total,
                disk,
                disk_used_gb: disk_used,
                disk_total_gb: disk_total,
                net_down: down,
                net_up: up,
                cpu_temp,
                gpu_temp,
                refresh_hz: refresh,
                screen_width,
                screen_height,
                cpu_name: hardware.cpu_name,
                gpu_name: hardware.gpu_name,
                motherboard: hardware.motherboard,
                ram_modules: hardware.ram_modules,
                os_name: hardware.os_name,
                fps: fps.fps,
                fps_process: fps.process,
                fps_error: fps.error,
                checked_at: chrono::Local::now().timestamp_millis(),
            };
            if let Ok(mut latest) = app.state::<StatsState>().latest.lock() {
                *latest = stats.clone();
            }
            let _ = app.emit("system-stats", &stats);
            std::thread::sleep(Duration::from_millis(1200));
        }
    });
}

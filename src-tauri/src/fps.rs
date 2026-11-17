use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use windows::core::PWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SKIP: &[&str] = &[
    "isle.exe",
    "explorer.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "applicationframehost.exe",
    "textinputhost.exe",
    "lockapp.exe",
    "dwm.exe",
    "code.exe",
    "cursor.exe",
    "windowsterminal.exe",
    "powershell.exe",
    "pwsh.exe",
    "cmd.exe",
    "msedge.exe",
    "chrome.exe",
    "firefox.exe",
    "notepad.exe",
];

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FpsInfo {
    pub fps: Option<f32>,
    pub process: String,
    pub error: String,
}

#[derive(Default)]
pub struct FpsState {
    info: Mutex<FpsInfo>,
}

impl FpsState {
    pub fn get(&self) -> FpsInfo {
        self.info.lock().map(|info| info.clone()).unwrap_or_default()
    }

    fn set(&self, next: FpsInfo) {
        if let Ok(mut info) = self.info.lock() {
            *info = next;
        }
    }
}

fn find_presentmon() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("ISLE_PRESENTMON") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(';') {
            let candidate = Path::new(dir).join("PresentMon.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let known = [
        r"C:\Program Files\UltraAPP\Assets\Tools\Latency\PresentMon.exe",
        r"C:\Program Files\Intel\PresentMon\PresentMon.exe",
        r"C:\Program Files\Intel\PresentMonSharedService\PresentMon.exe",
    ];
    for path in known {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn foreground_process() -> Option<(u32, String, bool)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        let (work_left, work_top, work_right, work_bottom) =
            crate::window_util::work_area_at((rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let work_width = work_right - work_left;
        let work_height = work_bottom - work_top;
        let fullscreen = work_width > 0
            && work_height > 0
            && width >= work_width * 95 / 100
            && height >= work_height * 95 / 100;

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if result.is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        let name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        Some((pid, name, fullscreen))
    }
}

fn parse_fps(stdout: &str) -> Option<f32> {
    let mut lines = stdout.lines();
    let header = lines.next()?;
    let columns: Vec<&str> = header.split(',').collect();
    let index = columns.iter().position(|column| {
        let name = column.trim().trim_matches('"');
        name == "msBetweenPresents" || name == "msBetweenDisplayChange"
    })?;
    let mut samples = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if let Some(value) = fields
            .get(index)
            .and_then(|value| value.trim().trim_matches('"').parse::<f64>().ok())
        {
            if value > 0.1 && value < 2000.0 {
                samples.push(value);
            }
        }
    }
    if samples.is_empty() {
        return None;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = samples[samples.len() / 2];
    if median <= 0.0 {
        return None;
    }
    Some((1000.0 / median).clamp(1.0, 1000.0) as f32)
}

fn capture(presentmon: &Path, pid: u32) -> Result<Option<f32>, String> {
    let pid_arg = pid.to_string();
    let output = Command::new(presentmon)
        .args([
            "--process_id",
            &pid_arg,
            "--timed",
            "1",
            "--terminate_after_timed",
            "--output_stdout",
            "--no_console_stats",
            "--v1_metrics",
            "--session_name",
            "IsleFps",
            "--stop_existing_session",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("access denied") || stderr.contains("elevated privilege") {
        return Err("admin".into());
    }
    if !output.status.success() {
        return Err("capture".into());
    }
    Ok(parse_fps(&String::from_utf8_lossy(&output.stdout)))
}

pub fn init(app: AppHandle) {
    app.manage(FpsState::default());
    let Some(presentmon) = find_presentmon() else {
        if let Some(state) = app.try_state::<FpsState>() {
            state.set(FpsInfo {
                error: "missing".into(),
                ..FpsInfo::default()
            });
        }
        return;
    };
    std::thread::spawn(move || loop {
        let Some(state) = app.try_state::<FpsState>() else {
            return;
        };
        let Some((pid, process, fullscreen)) = foreground_process() else {
            state.set(FpsInfo::default());
            std::thread::sleep(Duration::from_secs(3));
            continue;
        };
        let name = process.to_lowercase();
        if !fullscreen || SKIP.contains(&name.as_str()) {
            state.set(FpsInfo::default());
            std::thread::sleep(Duration::from_secs(3));
            continue;
        }
        match capture(&presentmon, pid) {
            Ok(fps) => state.set(FpsInfo {
                fps,
                process,
                error: String::new(),
            }),
            Err(error) => {
                let admin = error == "admin";
                state.set(FpsInfo {
                    fps: None,
                    process,
                    error: error.clone(),
                });
                if admin {
                    // Without elevation every further attempt fails the same
                    // way. Stop until the app is restarted as administrator.
                    return;
                }
            }
        }
        std::thread::sleep(Duration::from_secs(3));
    });
}

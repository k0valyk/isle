//! One-time elevated access.
//!
//! PresentMon reads ETW data, which Windows only exposes to elevated processes.
//! Asking the user to restart the app as administrator on every launch is the
//! wrong trade-off, so Isle registers a per-user scheduled task once (a single
//! UAC prompt) and relaunches itself through that task from then on. The task
//! runs with `RunLevel = Highest`, so no prompt is ever shown again.

use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub const TASK: &str = "Isle Elevated";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            // Fail safe: assume elevated so a broken token never loops the app.
            return true;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0u32;
        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        let _ = CloseHandle(token);
        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

fn schtasks(args: &[&str]) -> bool {
    Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn task_exists() -> bool {
    schtasks(&["/Query", "/TN", TASK])
}

fn quote(value: &str) -> String {
    value.replace('\'', "''")
}

/// Registers (or refreshes) the elevated task. Shows exactly one UAC prompt.
pub fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let script_path = std::env::temp_dir().join("isle-elevation-task.ps1");
    let script = format!(
        "$ErrorActionPreference='Stop'\n\
         $exe='{exe}'\n\
         $action=New-ScheduledTaskAction -Execute $exe -Argument '--elevated --logon'\n\
         $trigger=New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME\n\
         $principal=New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest\n\
         Register-ScheduledTask -TaskName '{task}' -Action $action -Trigger $trigger -Principal $principal -Description 'Isle elevated access: FPS and system sensors' -Force | Out-Null\n",
        exe = quote(&exe.display().to_string()),
        task = quote(TASK),
    );
    std::fs::write(&script_path, script).map_err(|error| error.to_string())?;

    let command = format!(
        "Start-Process -FilePath 'powershell' -Verb RunAs -Wait -ArgumentList \
         '-NoProfile','-ExecutionPolicy','Bypass','-File','{script}'",
        script = script_path.display()
    );
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&script_path);

    if !status.success() {
        return Err("elevation-cancelled".into());
    }
    if !task_exists() {
        return Err("elevation-failed".into());
    }
    Ok(())
}

/// Removes the task; falls back to an elevated delete when the user owns it.
pub fn disable() -> bool {
    if schtasks(&["/Delete", "/TN", TASK, "/F"]) {
        return true;
    }
    let command = format!(
        "Start-Process -FilePath 'schtasks' -Verb RunAs -Wait -ArgumentList '/Delete','/TN','{}','/F'",
        TASK
    );
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|status| status.success() && !task_exists())
        .unwrap_or(false)
}

/// Guards against a relaunch loop if the task ever loses its arguments.
pub fn should_handoff() -> bool {
    let stamp = std::env::temp_dir().join("isle-handoff.stamp");
    let recent = std::fs::metadata(&stamp)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.elapsed().ok())
        .map(|age| age.as_secs() < 20)
        .unwrap_or(false);
    if recent {
        return false;
    }
    let _ = std::fs::write(&stamp, b"1");
    true
}

/// Relaunches the app through the elevated task once the current process exits.
pub fn handoff() {    let pid = std::process::id();
    let script_path = std::env::temp_dir().join("isle-elevation-run.ps1");
    let script = format!(
        "param([int]$OldPid)\n\
        Wait-Process -Id $OldPid -ErrorAction SilentlyContinue\n\
        Start-Sleep -Milliseconds 450\n\
        schtasks /Run /TN '{task}' | Out-Null\n",
        task = quote(TASK),
    );
    if std::fs::write(&script_path, script).is_err() {
        return;
    }
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.display().to_string(),
            "-OldPid",
            &pid.to_string(),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

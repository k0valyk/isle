use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use base64::Engine as _;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

struct Session {
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
}

#[derive(Default)]
pub struct TerminalState {
    sessions: Mutex<HashMap<u32, Session>>,
    next: AtomicU32,
}

impl Drop for TerminalState {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for (_, mut session) in sessions.drain() {
                let _ = session.child.kill();
            }
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalChunk {
    id: u32,
    data: String,
}

fn spawn_shell(
    app: AppHandle,
    id: u32,
    cols: u16,
    rows: u16,
    state: &TerminalState,
) -> Result<(), String> {
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: rows.max(4),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())?;

    #[cfg(windows)]
    let shell = if std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path).any(|directory| directory.join("pwsh.exe").is_file())
        })
        .unwrap_or(false)
    {
        "pwsh.exe".to_string()
    } else {
        "powershell.exe".to_string()
    };
    #[cfg(not(windows))]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

    let mut command = CommandBuilder::new(&shell);
    #[cfg(windows)]
    command.args(["-NoLogo", "-NoExit"]);
    #[cfg(not(windows))]
    command.args(["-l"]);
    if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
        command.cwd(home);
    }
    command.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start {shell}: {error}"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| error.to_string())?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;

    {
        let mut sessions = state.sessions.lock().map_err(|error| error.to_string())?;
        sessions.insert(
            id,
            Session {
                writer,
                child,
                master: pair.master,
            },
        );
    }

    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let data = base64::engine::general_purpose::STANDARD
                        .encode(&buffer[..read]);
                    let _ = app.emit("terminal-output", TerminalChunk { id, data });
                }
                Err(_) => break,
            }
        }
        let _ = app.emit(
            "terminal-exit",
            TerminalChunk {
                id,
                data: String::new(),
            },
        );
    });

    Ok(())
}

#[tauri::command]
pub fn terminal_open(
    app: AppHandle,
    state: State<'_, TerminalState>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<u32, String> {
    let id = state.next.fetch_add(1, Ordering::Relaxed) + 1;
    spawn_shell(app, id, cols.unwrap_or(100), rows.unwrap_or(24), &state)?;
    Ok(id)
}

#[tauri::command]
pub fn terminal_write(
    state: State<'_, TerminalState>,
    id: u32,
    data: String,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|error| error.to_string())?;
    let session = sessions
        .get_mut(&id)
        .ok_or_else(|| "terminal session is not running".to_string())?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|error| error.to_string())?;
    session.writer.flush().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, TerminalState>,
    id: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|error| error.to_string())?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| "terminal session is not running".to_string())?;
    session
        .master
        .resize(PtySize {
            rows: rows.max(4),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn terminal_close(state: State<'_, TerminalState>, id: u32) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|error| error.to_string())?;
    if let Some(mut session) = sessions.remove(&id) {
        let _ = session.child.kill();
    }
    Ok(())
}

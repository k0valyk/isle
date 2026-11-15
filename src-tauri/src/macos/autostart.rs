//! macOS autostart through a per-user LaunchAgent.

use std::path::PathBuf;

fn plist_path() -> Option<PathBuf> {
    let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
    Some(
        home.join("Library")
            .join("LaunchAgents")
            .join("com.isle.app.plist"),
    )
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = plist_path().ok_or_else(|| "Could not locate the home directory".to_string())?;
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let parent = path.parent().ok_or_else(|| "Invalid LaunchAgents path".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.isle.app</string>
  <key>ProgramArguments</key>
  <array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><false/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        exe.to_string_lossy()
    );
    std::fs::write(&path, plist).map_err(|error| error.to_string())
}

pub fn installed() -> bool {
    plist_path().map(|path| path.exists()).unwrap_or(false)
}

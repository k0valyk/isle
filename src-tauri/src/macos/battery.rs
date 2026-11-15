//! macOS battery readout via `pmset -g batt`.

use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStatus {
    pub present: bool,
    pub percent: Option<u8>,
    pub charging: bool,
    pub ac: bool,
}

#[tauri::command]
pub fn battery_status() -> BatteryStatus {
    let output = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .ok();

    let text = output
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .unwrap_or_default();

    if !text.contains("InternalBattery") {
        return BatteryStatus {
            present: false,
            percent: None,
            charging: false,
            ac: true,
        };
    }

    let percent = text
        .split('%')
        .next()
        .and_then(|head| head.rsplit(|c: char| !c.is_ascii_digit()).next())
        .and_then(|value| value.parse::<u8>().ok());
    let charging = text.contains("charging") || text.contains("charged");
    let ac = text.contains("AC Power");

    BatteryStatus {
        present: true,
        percent,
        charging,
        ac,
    }
}

use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[link(name = "wininet")]
extern "system" {
    fn InternetGetConnectedState(flags: *mut u32, reserved: u32) -> i32;
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkAdapter {
    pub name: String,
    pub description: String,
    pub status: String,
    pub speed: String,
    pub ipv4: Vec<String>,
    pub kind: String,
    pub vpn: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub online: bool,
    pub wifi_ssid: Option<String>,
    pub vpn: Option<String>,
    pub adapters: Vec<NetworkAdapter>,
    pub checked_at: i64,
}

#[derive(Deserialize)]
struct RawAdapter {
    #[serde(default, rename = "Name")]
    name: String,
    #[serde(default, rename = "InterfaceDescription")]
    description: String,
    #[serde(default, rename = "Status")]
    status: String,
    #[serde(default, rename = "LinkSpeed")]
    speed: String,
    #[serde(default, rename = "ifIndex")]
    if_index: i64,
}

#[derive(Deserialize)]
struct RawIp {
    #[serde(default, rename = "ifIndex")]
    if_index: i64,
    #[serde(default, rename = "IPAddress")]
    address: String,
}

#[derive(Deserialize)]
struct RawNetwork {
    #[serde(default)]
    adapters: Vec<RawAdapter>,
    #[serde(default)]
    ips: Vec<RawIp>,
}

const VPN_MARKERS: [&str; 13] = [
    "vpn",
    "wireguard",
    "openvpn",
    "wintun",
    "tap-",
    "tap ",
    "tailscale",
    "zerotier",
    "proton",
    "nordlynx",
    "surfshark",
    "mullvad",
    "anyconnect",
];

fn is_vpn(description: &str) -> bool {
    let lower = description.to_lowercase();
    VPN_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn adapter_kind(description: &str) -> &'static str {
    let lower = description.to_lowercase();
    if is_vpn(description) {
        "vpn"
    } else if lower.contains("wi-fi")
        || lower.contains("wifi")
        || lower.contains("wireless")
        || lower.contains("802.11")
        || lower.contains("wlan")
    {
        "wifi"
    } else if lower.contains("ethernet") || lower.contains("gbe") || lower.contains("pcie") {
        "ethernet"
    } else {
        "other"
    }
}

fn run_powershell(script: &str) -> Result<String, String> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|error| format!("PowerShell is unavailable: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Network query failed: {}", error.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn wifi_ssid() -> Option<String> {
    let output = std::process::Command::new("netsh")
        .args(["wlan", "show", "interfaces"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_lowercase().starts_with("ssid") {
            continue;
        }
        if trimmed.to_ascii_lowercase().starts_with("bssid") {
            continue;
        }
        if let Some((_, value)) = trimmed.split_once(':') {
            let ssid = value.trim();
            if !ssid.is_empty() {
                return Some(ssid.to_string());
            }
        }
    }
    None
}

fn collect() -> Result<NetworkStatus, String> {
    let script = r#"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $adapters = @(Get-NetAdapter | Select-Object Name,InterfaceDescription,Status,LinkSpeed,ifIndex); $ips = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } | Select-Object ifIndex,IPAddress); @{ adapters = $adapters; ips = $ips } | ConvertTo-Json -Depth 4 -Compress"#;
    let raw = run_powershell(script)?;
    let parsed: RawNetwork = serde_json::from_str(&raw)
        .map_err(|error| format!("Network response is invalid: {error}"))?;

    let adapters: Vec<NetworkAdapter> = parsed
        .adapters
        .into_iter()
        .map(|adapter| {
            let ipv4: Vec<String> = parsed
                .ips
                .iter()
                .filter(|ip| ip.if_index == adapter.if_index)
                .map(|ip| ip.address.clone())
                .collect();
            NetworkAdapter {
                name: adapter.name,
                description: adapter.description.clone(),
                status: adapter.status,
                speed: adapter.speed,
                ipv4,
                kind: adapter_kind(&adapter.description).into(),
                vpn: is_vpn(&adapter.description),
            }
        })
        .collect();

    let vpn = adapters
        .iter()
        .find(|adapter| adapter.vpn && adapter.status.eq_ignore_ascii_case("up"))
        .map(|adapter| adapter.name.clone())
        .or_else(|| adapters.iter().find(|adapter| adapter.vpn).map(|adapter| adapter.name.clone()));

    let online = unsafe {
        let mut flags = 0u32;
        InternetGetConnectedState(&mut flags, 0) != 0
    };

    Ok(NetworkStatus {
        online,
        wifi_ssid: wifi_ssid(),
        vpn,
        adapters,
        checked_at: chrono::Local::now().timestamp_millis(),
    })
}

#[tauri::command]
pub async fn network_status() -> Result<NetworkStatus, String> {
    tauri::async_runtime::spawn_blocking(collect)
        .await
        .map_err(|error| error.to_string())?
}

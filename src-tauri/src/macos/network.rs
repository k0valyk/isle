//! macOS network status.
//!
//! Adapter enumeration lives in `networksetup`, which is slow and localised;
//! the island only needs an online flag for the weather/chat widgets, so a
//! lightweight TCP probe is enough for the first macOS build.

use serde::Serialize;

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

fn online() -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let Some(address) = "1.1.1.1:443".to_socket_addrs().ok().and_then(|mut list| list.next())
    else {
        return false;
    };
    TcpStream::connect_timeout(&address, Duration::from_millis(900)).is_ok()
}

#[tauri::command]
pub async fn network_status() -> Result<NetworkStatus, String> {
    tauri::async_runtime::spawn_blocking(|| NetworkStatus {
        online: online(),
        wifi_ssid: None,
        vpn: None,
        adapters: Vec::new(),
        checked_at: chrono::Local::now().timestamp_millis(),
    })
    .await
    .map_err(|error| error.to_string())
}

use std::collections::HashSet;
use std::time::Duration;

use tetris_net::lan_discovery::LanDiscovery;

const SCAN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(serde::Serialize)]
struct RelayInfo {
    label: String,
    ws_url: String,
    version: String,
}

/// Scan the LAN for relay servers advertising `_tetris._udp.local.`. Runs the
/// blocking mDNS browse off the async runtime and dedups by resolved ws URL.
#[tauri::command]
async fn lan_discover() -> Result<Vec<RelayInfo>, String> {
    let hosts = tauri::async_runtime::spawn_blocking(|| {
        let discovery = LanDiscovery::new().map_err(|e| e.to_string())?;
        discovery.browse(SCAN_TIMEOUT).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let mut seen = HashSet::new();
    let mut relays = Vec::new();
    for host in hosts {
        let ws_url = format!("ws://{}", host.address);
        if seen.insert(ws_url.clone()) {
            relays.push(RelayInfo {
                label: host.label,
                ws_url,
                version: host.version,
            });
        }
    }
    Ok(relays)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![lan_discover])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use tetris_net::error::NetError;
use tetris_net::lan_discovery::LanDiscovery;
use tetris_relay::logging::init_logging;
use tetris_relay::relay::RoomManager;
use tetris_relay::ws_handler::{AppState, ws_handler};

#[tokio::main]
async fn main() {
    init_logging(parse_log_file());

    let port = parse_port();
    let state = Arc::new(AppState {
        room_manager: Arc::new(RoomManager::new(100)),
        pending_inputs: Arc::new(Mutex::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/room/{code}", get(ws_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("tetris-relay listening on {addr}");

    // Announce on the LAN via mDNS so desktop clients can auto-discover this
    // relay. Held until shutdown (Drop unregisters). `--no-lan` opts out.
    let _lan = if parse_no_lan() {
        info!("LAN mDNS announce disabled (--no-lan)");
        None
    } else {
        match announce_lan(port) {
            Ok(discovery) => Some(discovery),
            Err(e) => {
                warn!("LAN mDNS announce failed: {e}");
                None
            }
        }
    };

    let listener = TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        error!("failed to bind {addr}: {e}");
        std::process::exit(1);
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            error!("server error: {e}");
            std::process::exit(1);
        });
}

fn parse_port() -> u16 {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port"
            && let Some(p) = args.get(i + 1)
        {
            return p.parse().unwrap_or(9000);
        }
    }
    9000
}

fn parse_log_file() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--log-file"
            && let Some(path) = args.get(i + 1)
        {
            return Some(PathBuf::from(path));
        }
    }
    None
}

fn parse_no_lan() -> bool {
    std::env::args().any(|arg| arg == "--no-lan")
}

fn relay_label(port: u16) -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Tetris Relay".to_string());
    format!("{host}:{port}")
}

fn announce_lan(port: u16) -> Result<LanDiscovery, NetError> {
    let mut discovery = LanDiscovery::new()?;
    let label = relay_label(port);
    discovery.publish_relay(port, &label)?;
    info!("LAN mDNS announced as '{label}'");
    Ok(discovery)
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

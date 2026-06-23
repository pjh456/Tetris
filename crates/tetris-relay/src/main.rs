use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info};

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

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

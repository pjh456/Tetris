use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

use tetris_relay::relay::RoomManager;
use tetris_relay::ws_handler::{AppState, ws_handler};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = parse_port();
    let state = Arc::new(AppState {
        room_manager: Arc::new(RoomManager::new(100)),
    });

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/room/{code}", get(ws_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    info!("tetris-relay listening on {addr}");

    let listener = TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        eprintln!("Failed to bind {addr}: {e}");
        std::process::exit(1);
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            eprintln!("Server error: {e}");
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

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

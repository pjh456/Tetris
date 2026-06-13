use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{info, warn};

use crate::relay::RoomManager;

pub struct AppState {
    pub room_manager: Arc<RoomManager>,
}

#[allow(clippy::unused_async)]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(room_code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let code = room_code.to_uppercase();
    ws.on_upgrade(move |socket| handle_socket(socket, code, state))
}

async fn handle_socket(socket: WebSocket, room_code: String, state: Arc<AppState>) {
    if let Err(e) = state.room_manager.get_or_create_room(&room_code).await {
        warn!("get_or_create_room failed: {e}");
        return;
    }

    let rx = match state.room_manager.join_room(&room_code).await {
        Ok(rx) => rx,
        Err(e) => {
            warn!("join_room failed: {e}");
            return;
        }
    };

    info!("client joined room {room_code}");

    let (sender, mut receiver) = socket.split();
    let room_code_send = room_code.clone();

    let send_task = tokio::spawn(send_loop(sender, rx));

    while let Some(msg_result) = receiver.next().await {
        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                warn!("recv error: {e}");
                break;
            }
        };

        match msg {
            Message::Binary(data) => {
                let _ = state
                    .room_manager
                    .broadcast(&room_code, data.to_vec())
                    .await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    state.room_manager.leave_room(&room_code).await;
    info!("client left room {room_code_send}");
}

async fn send_loop(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut rx: broadcast::Receiver<Vec<u8>>,
) {
    let mut ping_interval = interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(data) => {
                        if sender.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {},
                }
            }
            _ = ping_interval.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

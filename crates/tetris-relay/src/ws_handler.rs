use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use bincode::{deserialize, serialize};
use futures_util::{SinkExt, StreamExt};
use tetris_protocol::protocol::{
    PROTOCOL_VERSION, PacketHeader, PacketType, PktChatMessage, PktGameStart, PktJoinRoom,
    PktPlayerReady, PktServerAccept, PktStartCountdown,
};
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

    let peer_id = RoomManager::alloc_peer_id();

    let peers = match state.room_manager.add_peer(&room_code, peer_id).await {
        Ok(p) => p,
        Err(e) => {
            warn!("add_peer failed: {e}");
            state.room_manager.leave_room(&room_code).await;
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let room_code_send = room_code.clone();

    if let Ok(peer) = state.room_manager.peer_by_id(&room_code, peer_id).await {
        let accept = PktServerAccept {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ServerAccept,
                player_id: 0,
            },
            assigned_player_id: peer.player_id,
            max_players: 4,
        };
        if let Ok(data) = serialize(&accept) {
            let _ = sender.send(Message::Binary(data.into())).await;
        }
    }

    info!("client {peer_id} joined room {room_code}");
    state
        .room_manager
        .broadcast_snapshot(&room_code, &peers)
        .await;

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
                handle_binary_message(&state, &room_code, peer_id, data.to_vec()).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();

    let remaining = state.room_manager.remove_peer(&room_code, peer_id).await;
    state.room_manager.leave_room(&room_code).await;
    info!("client {peer_id} left room {room_code_send}");

    if !remaining.is_empty() {
        state
            .room_manager
            .broadcast_snapshot(&room_code_send, &remaining)
            .await;
    }
}

async fn handle_binary_message(
    state: &Arc<AppState>,
    room_code: &str,
    peer_id: u64,
    data: Vec<u8>,
) {
    let header = match deserialize::<PacketHeader>(&data) {
        Ok(header) if header.version == PROTOCOL_VERSION => header,
        _ => {
            let _ = state.room_manager.broadcast(room_code, data).await;
            return;
        }
    };

    match header.packet_type {
        PacketType::JoinRoom => {
            let Ok(pkt) = deserialize::<PktJoinRoom>(&data) else {
                return;
            };
            if let Ok(peers) = state
                .room_manager
                .rename_peer(room_code, peer_id, pkt.player_name.clone())
                .await
            {
                state
                    .room_manager
                    .broadcast_snapshot(room_code, &peers)
                    .await;
            }
        }
        PacketType::PlayerReady => {
            let Ok(pkt) = deserialize::<PktPlayerReady>(&data) else {
                return;
            };
            if let Ok(peers) = state
                .room_manager
                .set_peer_ready(room_code, peer_id, pkt.ready)
                .await
            {
                state
                    .room_manager
                    .broadcast_snapshot(room_code, &peers)
                    .await;
                let all_ready = peers.len() >= 2 && peers.iter().all(|peer| peer.ready);
                let countdown_active = state
                    .room_manager
                    .countdown_active(room_code)
                    .await
                    .unwrap_or(false);
                if all_ready && !countdown_active {
                    let _ = state
                        .room_manager
                        .set_countdown_active(room_code, true)
                        .await;
                    let state = Arc::clone(state);
                    let room_code = room_code.to_string();
                    tokio::spawn(async move {
                        for remaining_secs in [3_u8, 2, 1, 0] {
                            let countdown = PktStartCountdown {
                                header: PacketHeader {
                                    version: PROTOCOL_VERSION,
                                    packet_type: PacketType::StartCountdown,
                                    player_id: 0,
                                },
                                remaining_secs,
                            };
                            if let Ok(data) = serialize(&countdown) {
                                let _ = state.room_manager.broadcast(&room_code, data).await;
                            }
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        let random_seed = rand::random::<u32>();
                        let game_start = PktGameStart {
                            header: PacketHeader {
                                version: PROTOCOL_VERSION,
                                packet_type: PacketType::GameStart,
                                player_id: 0,
                            },
                            random_seed,
                        };
                        if let Ok(data) = serialize(&game_start) {
                            let _ = state.room_manager.broadcast(&room_code, data).await;
                        }
                        if let Ok(reset_peers) =
                            state.room_manager.reset_ready_states(&room_code).await
                        {
                            let _ = state
                                .room_manager
                                .set_countdown_active(&room_code, false)
                                .await;
                            state
                                .room_manager
                                .broadcast_snapshot(&room_code, &reset_peers)
                                .await;
                        }
                    });
                }
            }
        }
        PacketType::ChatMessage => {
            let Ok(pkt) = deserialize::<PktChatMessage>(&data) else {
                return;
            };
            let chat_pkt = if let Ok(peer) = state.room_manager.peer_by_id(room_code, peer_id).await
            {
                PktChatMessage {
                    header: PacketHeader {
                        version: PROTOCOL_VERSION,
                        packet_type: PacketType::ChatMessage,
                        player_id: peer.player_id,
                    },
                    message: pkt.message,
                    timestamp: pkt.timestamp,
                }
            } else {
                pkt
            };
            if let Ok(data) = serialize(&chat_pkt) {
                let _ = state.room_manager.broadcast(room_code, data).await;
            }
        }
        _ => {
            let _ = state.room_manager.broadcast(room_code, data).await;
        }
    }
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

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use bincode::{deserialize, serialize};
use futures_util::{SinkExt, StreamExt};
use tetris_protocol::protocol::{
    PROTOCOL_VERSION, PacketHeader, PacketType, PktChatMessage, PktGameStart, PktJoinRoom,
    PktPlayerReady, PktReplay, PktServerAccept, PktStartCountdown, InputEvent,
};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{info, warn};

use crate::relay::RoomManager;
use crate::room_actor::RoomActor;
use tetris_protocol::newtypes::Seed;

use std::collections::HashMap;

type PlayerChannel = (u8, tokio::sync::mpsc::Receiver<InputEvent>, tokio::sync::mpsc::Sender<Vec<u8>>);

pub struct AppState {
    pub room_manager: Arc<RoomManager>,
    pub pending_inputs: Arc<Mutex<HashMap<String, Vec<PlayerChannel>>>>,
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

    let _broadcast_rx = match state.room_manager.join_room(&room_code).await {
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

    // Per-client bounded input + outbound channels for RoomActor (D-18)
    let (input_tx, input_rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
    let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    if let Ok(peer) = state.room_manager.peer_by_id(&room_code, peer_id).await {
        // Store input_rx + outbound_tx for RoomActor when game starts
        {
            let mut pending = state.pending_inputs.lock().await;
            pending
                .entry(room_code.clone())
                .or_default()
                .push((peer.player_id, input_rx, outbound_tx));
        }

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

    let send_task = tokio::spawn(send_loop(sender, outbound_rx));

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
                handle_binary_message(&state, &room_code, peer_id, &input_tx, data.to_vec()).await;
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
    input_tx: &tokio::sync::mpsc::Sender<InputEvent>,
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
                    let room_code_owned = room_code.to_string();
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
                                let _ = state.room_manager.broadcast(&room_code_owned, data).await;
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
                            let _ = state.room_manager.broadcast(&room_code_owned, data).await;
                        }
                        // Retrieve pending channels and spawn RoomActor
                        let player_channels: Vec<PlayerChannel> = {
                            let mut pending = state.pending_inputs.lock().await;
                            pending.remove(&room_code_owned).unwrap_or_default()
                        };
                        if !player_channels.is_empty() {
                            let mut actor = RoomActor::new(
                                room_code_owned.clone(),
                                Seed(random_seed as i32),
                            );
                            for (player_id, input_rx, outbound_tx) in player_channels {
                                use crate::player_conn::PlayerConnection;
                                use crate::player_conn::Online;
                                use tetris_protocol::newtypes::PlayerSlot;
                                let (conn_tx, _) = tokio::sync::mpsc::channel::<InputEvent>(64);
                                let conn = PlayerConnection::<Online>::new(
                                    PlayerSlot(player_id),
                                    conn_tx,
                                    format!("Player {}", player_id + 1),
                                );
                                actor.add_player(PlayerSlot(player_id), input_rx, conn, outbound_tx);
                            }
                            let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
                            tokio::spawn(actor.run(cancel_rx));
                        }

                        if let Ok(reset_peers) =
                            state.room_manager.reset_ready_states(&room_code_owned).await
                        {
                            let _ = state
                                .room_manager
                                .set_countdown_active(&room_code_owned, false)
                                .await;
                            state
                                .room_manager
                                .broadcast_snapshot(&room_code_owned, &reset_peers)
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
        // New protocol types (Replay, etc.) — forward to input_tx if RoomActor is active
        PacketType::Replay | PacketType::PlayerAction => {
            if let Ok(pkt) = deserialize::<PktReplay>(&data) {
                for ev in &pkt.events {
                    let _ = input_tx.try_send(ev.clone());
                }
            } else {
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
    mut rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
) {
    let mut ping_interval = interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Some(data) => {
                        if sender.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
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

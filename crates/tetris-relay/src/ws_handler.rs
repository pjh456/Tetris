use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use bincode::Options;
use bincode::serialize;
use futures_util::{SinkExt, StreamExt};
use tetris_protocol::protocol::{
    InputEvent, PROTOCOL_VERSION, PacketHeader, PacketType, PktChatMessage, PktGameStart,
    PktJoinRoom, PktPlayerReady, PktReplay, PktServerAccept, PktStartCountdown,
};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{info, warn};

use crate::relay::RoomManager;
use crate::room_actor::RoomActor;
use tetris_protocol::newtypes::Seed;
use tetris_sim::RoomMode;

use std::collections::HashMap;

const MAX_PACKET_BYTES: u64 = 65536;
const MAX_REPLAY_EVENTS_PER_PACKET: usize = 120;
const MAX_REPLAY_TICK_SPAN: u64 = 120;
const MAX_MSG_PER_SEC: u32 = 60;
const MAX_CHAT_LEN: usize = 256;

fn deser<'de, T: serde::Deserialize<'de>>(data: &'de [u8]) -> Result<T, bincode::Error> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(MAX_PACKET_BYTES)
        .deserialize::<T>(data)
}

type PlayerChannel = (
    u8,
    tokio::sync::mpsc::Receiver<InputEvent>,
    tokio::sync::mpsc::Sender<Vec<u8>>,
);

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
    if room_code.len() != 4 || !room_code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return;
    }
    if let Err(e) = state.room_manager.get_or_create_room(&room_code).await {
        warn!("get_or_create_room failed: {e}");
        return;
    }

    let broadcast_rx = match state.room_manager.join_room(&room_code).await {
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
            pending.entry(room_code.clone()).or_default().push((
                peer.player_id,
                input_rx,
                outbound_tx,
            ));
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
    } else {
        warn!("peer_by_id failed for {peer_id} in room {room_code}");
        return;
    }

    info!("client {peer_id} joined room {room_code}");
    state
        .room_manager
        .broadcast_snapshot(&room_code, &peers)
        .await;

    let send_task = tokio::spawn(send_loop(sender, outbound_rx, broadcast_rx));

    let mut msg_count: u32 = 0;
    let mut msg_window_start = tokio::time::Instant::now();

    while let Some(msg_result) = receiver.next().await {
        let now = tokio::time::Instant::now();
        if now - msg_window_start > Duration::from_secs(1) {
            msg_count = 0;
            msg_window_start = now;
        }
        msg_count += 1;
        if msg_count > MAX_MSG_PER_SEC {
            continue;
        }

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

    // Remove pending_inputs entry for this peer to avoid ghost player in RoomActor
    if let Ok(peer) = state.room_manager.peer_by_id(&room_code, peer_id).await {
        let mut pending = state.pending_inputs.lock().await;
        if let Some(entries) = pending.get_mut(&room_code) {
            entries.retain(|(pid, _, _)| *pid != peer.player_id);
        }
    }

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

/// NOTE: each packet is deserialized twice — first as `PacketHeader` for dispatch,
/// then as the typed packet. Acceptable for relay; would optimize for hot path.
async fn handle_binary_message(
    state: &Arc<AppState>,
    room_code: &str,
    peer_id: u64,
    input_tx: &tokio::sync::mpsc::Sender<InputEvent>,
    data: Vec<u8>,
) {
    if data.len() > MAX_PACKET_BYTES as usize {
        return;
    }

    let header = match deser::<PacketHeader>(&data) {
        Ok(header) if header.version == PROTOCOL_VERSION => header,
        _ => return,
    };

    match header.packet_type {
        PacketType::JoinRoom => {
            let Ok(pkt) = deser::<PktJoinRoom>(&data) else {
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
            let Ok(pkt) = deser::<PktPlayerReady>(&data) else {
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
                if !all_ready {
                    let _ = state
                        .room_manager
                        .set_countdown_active(room_code, false)
                        .await;
                }
                if all_ready
                    && state
                        .room_manager
                        .try_start_countdown(room_code)
                        .await
                        .unwrap_or(false)
                {
                    let state = Arc::clone(state);
                    let room_code_owned = room_code.to_string();
                    tokio::spawn(async move {
                        for remaining_secs in [3_u8, 2, 1, 0] {
                            let still_ready = state
                                .room_manager
                                .all_peers_ready(&room_code_owned)
                                .await
                                .unwrap_or(false);
                            let countdown_active = state
                                .room_manager
                                .countdown_active(&room_code_owned)
                                .await
                                .unwrap_or(false);
                            if !still_ready || !countdown_active {
                                let _ = state
                                    .room_manager
                                    .set_countdown_active(&room_code_owned, false)
                                    .await;
                                return;
                            }
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
                        if !state
                            .room_manager
                            .all_peers_ready(&room_code_owned)
                            .await
                            .unwrap_or(false)
                        {
                            let _ = state
                                .room_manager
                                .set_countdown_active(&room_code_owned, false)
                                .await;
                            return;
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
                            let mut actor =
                                RoomActor::new(room_code_owned.clone(), Seed(random_seed as i32));
                            for (player_id, input_rx, outbound_tx) in player_channels {
                                use crate::player_conn::Online;
                                use crate::player_conn::PlayerConnection;
                                use tetris_protocol::newtypes::PlayerSlot;
                                // conn_tx is intentionally a dropped channel — input path goes
                                // through input_rx from the connection, not PlayerConnection::send_input
                                let (conn_tx, _) = tokio::sync::mpsc::channel::<InputEvent>(64);
                                let conn = PlayerConnection::<Online>::new(
                                    PlayerSlot(player_id),
                                    conn_tx,
                                    format!("Player {}", player_id + 1),
                                );
                                actor.add_player(
                                    PlayerSlot(player_id),
                                    input_rx,
                                    conn,
                                    outbound_tx,
                                );
                            }
                            actor.set_room_mode(RoomMode::Playing);
                            let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
                            tokio::spawn(actor.run(cancel_rx));
                            // cancel_tx stored in RoomState for lifecycle; currently only
                            // dropped by room removal — no explicit RoomActor shutdown call site.
                            let _ = state
                                .room_manager
                                .store_cancel_tx(&room_code_owned, cancel_tx)
                                .await;
                        }

                        if let Ok(reset_peers) = state
                            .room_manager
                            .reset_ready_states(&room_code_owned)
                            .await
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
            let Ok(mut pkt) = deser::<PktChatMessage>(&data) else {
                return;
            };
            pkt.message.truncate(MAX_CHAT_LEN);
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
                return;
            };
            if let Ok(data) = serialize(&chat_pkt) {
                let _ = state.room_manager.broadcast(room_code, data).await;
            }
        }
        // New protocol types (Replay, etc.) — forward to input_tx if RoomActor is active
        PacketType::Replay => {
            if let Ok(pkt) = deser::<PktReplay>(&data)
                && replay_packet_is_valid(&pkt)
            {
                for ev in pkt.events {
                    let _ = input_tx.try_send(ev);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_protocol::newtypes::{KeyAction, TickNumber};

    fn make_replay(player_id: u8, events: Vec<InputEvent>) -> PktReplay {
        PktReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Replay,
                player_id,
            },
            events,
            start_tick: TickNumber(10),
        }
    }

    fn make_event(tick: u64) -> InputEvent {
        InputEvent {
            key: KeyAction::KeyHardDrop,
            pressed: true,
            tick: TickNumber(tick),
            subframe: 0.5,
        }
    }

    #[tokio::test]
    async fn replay_input_routes_by_connection_slot() {
        let manager = Arc::new(RoomManager::new(4));
        manager.get_or_create_room("ABCD").await.unwrap();
        let state = Arc::new(AppState {
            room_manager: manager,
            pending_inputs: Arc::new(Mutex::new(HashMap::new())),
        });
        let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<InputEvent>(4);
        let pkt = make_replay(99, vec![make_event(10)]);
        let data = serialize(&pkt).unwrap();

        handle_binary_message(&state, "ABCD", 1, &input_tx, data).await;

        let received = input_rx.try_recv().unwrap();
        assert_eq!(received.key, KeyAction::KeyHardDrop);
    }

    #[test]
    fn replay_packet_rejects_invalid_tick_window() {
        let pkt = make_replay(0, vec![make_event(131)]);

        assert!(!replay_packet_is_valid(&pkt));
    }
}

fn replay_packet_is_valid(pkt: &PktReplay) -> bool {
    if pkt.events.is_empty() || pkt.events.len() > MAX_REPLAY_EVENTS_PER_PACKET {
        return false;
    }

    let max_tick = pkt.start_tick.0.saturating_add(MAX_REPLAY_TICK_SPAN);
    pkt.events.iter().all(|event| {
        event.tick >= pkt.start_tick
            && event.tick.0 <= max_tick
            && event.subframe.is_finite()
            && (0.0..1.0).contains(&event.subframe)
    })
}

async fn send_loop(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut outbound_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    mut broadcast_rx: tokio::sync::broadcast::Receiver<Vec<u8>>,
) {
    let mut ping_interval = interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            result = outbound_rx.recv() => {
                match result {
                    Some(data) => {
                        if sender.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            result = broadcast_rx.recv() => {
                match result {
                    Ok(data) => {
                        if sender.send(Message::Binary(data.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
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

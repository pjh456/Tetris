use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use bincode::Options;
use bincode::serialize;
use futures_util::{SinkExt, StreamExt};
use tetris_protocol::protocol::{
    InputEvent, PROTOCOL_VERSION, PacketHeader, PacketType, PktAddBot, PktChatMessage, PktConnect,
    PktGameStart, PktJoinRoom, PktKickPlayer, PktPlayerReady, PktReconnect, PktRemoveBot,
    PktReplay, PktResume, PktRoomSettings, PktServerAccept, PktStartCountdown,
};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::relay::{MAX_PLAYERS_PER_ROOM, RelaySettings, RoomManager};
use crate::room_actor::{RoomActor, RoomCommand};
use tetris_core::engine::RoomRules;
use tetris_protocol::newtypes::{PlayerSlot, Seed};
use tetris_sim::RoomMode;

use std::collections::HashMap;

const MAX_PACKET_BYTES: u64 = 65536;
const MAX_REPLAY_EVENTS_PER_PACKET: usize = 120;
const MAX_REPLAY_TICK_SPAN: u64 = 120;
const MAX_MSG_PER_SEC: u32 = 60;
const MAX_CHAT_LEN: usize = 256;
const MAX_INITIAL_GARBAGE_LINES: u8 = 12;
const MAX_GARBAGE_DELAY_TICKS: u16 = 600;
/// Grace window after an in-game disconnect before the slot/engine is removed.
/// A reconnect with a valid resume token within this window reclaims the slot.
const RECONNECT_GRACE_SECS: u64 = 10;

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

    let (sender, mut receiver) = socket.split();
    let room_code_send = room_code.clone();

    // Per-client bounded input + outbound channels for RoomActor (D-18).
    let (input_tx, input_rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
    let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    // send_loop owns the socket sender; ServerAccept + gamestate all flow through
    // outbound_tx, so the handshake never touches the raw sink directly.
    let send_task = tokio::spawn(send_loop(sender, outbound_rx, broadcast_rx));

    // Handshake: the client's first packet is always an explicit `PktConnect`
    // declaring intent — empty resume_token = fresh join, non-empty = resume
    // attempt. No peek/timeout heuristic: the WS is already open so this arrives
    // immediately, and the intent is stated rather than inferred from packet type.
    let connect_bytes = loop {
        match receiver.next().await {
            Some(Ok(Message::Binary(data))) if !data.is_empty() => break data.to_vec(),
            Some(Ok(_)) => {}
            Some(Err(e)) => {
                warn!("handshake recv error from {peer_id}: {e}");
                send_task.abort();
                state.room_manager.leave_room(&room_code).await;
                return;
            }
            None => {
                send_task.abort();
                state.room_manager.leave_room(&room_code).await;
                return;
            }
        }
    };
    let Ok(connect) = deser::<PktConnect>(&connect_bytes) else {
        warn!("handshake: first packet from {peer_id} was not a PktConnect; closing");
        send_task.abort();
        state.room_manager.leave_room(&room_code).await;
        return;
    };

    let reclaimed = if connect.resume_token.is_empty() {
        None
    } else {
        state
            .room_manager
            .reclaim_away_peer(&room_code, peer_id, &connect.resume_token)
            .await
    };

    let peer = if let Some(peer) = reclaimed {
        info!(
            "client {peer_id} reclaimed slot {} in room {room_code}",
            peer.session.player_id
        );
        peer
    } else {
        // Fresh join (or a forged/stale resume token → no hijack, join anew).
        if let Err(e) = state.room_manager.add_peer(&room_code, peer_id).await {
            warn!("add_peer failed: {e}");
            // Explicit rejection so the client isn't left guessing on a silent
            // close: a ServerAccept with assigned_player_id = u8::MAX signals
            // "connection rejected" (room full / all away).
            let reject = PktServerAccept {
                header: PacketHeader::new(PacketType::ServerAccept, 0),
                assigned_player_id: u8::MAX,
                max_players: 0,
                resume_token: String::new(),
            };
            if let Ok(data) = serialize(&reject) {
                let _ = outbound_tx.send(data).await;
                // Give send_loop a moment to flush the rejection before abort.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            send_task.abort();
            state.room_manager.leave_room(&room_code).await;
            return;
        }
        match state.room_manager.peer_by_id(&room_code, peer_id).await {
            Ok(peer) => peer,
            Err(e) => {
                warn!("peer_by_id failed for {peer_id} in room {room_code}: {e}");
                send_task.abort();
                state.room_manager.leave_room(&room_code).await;
                return;
            }
        }
    };

    // Bind this connection's channels to the resolved slot. When a RoomActor is
    // already running, ResumePlayer rebuilds that slot's outbound/input channels
    // (preserving the engine on reclaim); otherwise the channels wait in pending
    // until the game starts.
    let conn = make_player_connection(peer.session.player_id, input_tx.clone());
    if let Ok(Some(actor_tx)) = state.room_manager.actor_tx(&room_code).await {
        let _ = actor_tx
            .send(RoomCommand::ResumePlayer {
                slot: PlayerSlot(peer.session.player_id),
                input_rx,
                conn,
                outbound_tx: outbound_tx.clone(),
            })
            .await;
    } else {
        let mut pending = state.pending_inputs.lock().await;
        pending.entry(room_code.clone()).or_default().push((
            peer.session.player_id,
            input_rx,
            outbound_tx.clone(),
        ));
    }

    // ServerAccept carries the server-issued resume_token. Sent over outbound_tx
    // so only this client receives it (never in a room-wide broadcast).
    let accept = PktServerAccept {
        header: PacketHeader::new(PacketType::ServerAccept, 0),
        assigned_player_id: peer.session.player_id,
        max_players: MAX_PLAYERS_PER_ROOM as u8,
        resume_token: peer.session.resume_token.clone(),
    };
    if let Ok(data) = serialize(&accept) {
        let _ = outbound_tx.send(data).await;
    }

    info!("client {peer_id} joined room {room_code}");
    state.room_manager.broadcast_room_snapshot(&room_code).await;

    let mut msg_count: u32 = 0;
    let mut msg_window_start = tokio::time::Instant::now();

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
                let now = tokio::time::Instant::now();
                if now - msg_window_start > Duration::from_secs(1) {
                    msg_count = 0;
                    msg_window_start = now;
                }
                msg_count += 1;
                if msg_count > MAX_MSG_PER_SEC && !binary_packet_is_replay(&data) {
                    continue;
                }
                handle_binary_message(&state, &room_code, peer_id, &input_tx, data.to_vec()).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();

    let peer = state
        .room_manager
        .peer_by_id(&room_code, peer_id)
        .await
        .ok();

    // Always drop any pending (pre-game) channel for this peer.
    if let Some(peer) = &peer {
        let mut pending = state.pending_inputs.lock().await;
        if let Some(entries) = pending.get_mut(&room_code) {
            entries.retain(|(pid, _, _)| *pid != peer.session.player_id);
        }
    }

    let actor_present = matches!(state.room_manager.actor_tx(&room_code).await, Ok(Some(_)));

    if let (Some(peer), true) = (&peer, actor_present) {
        // In-game disconnect: enter a grace window instead of removing immediately.
        // The slot + engine are kept so a reconnect (valid resume token) reclaims
        // them; the player is only truly removed if still away after the grace
        // period — distinguishing a brief drop from leaving for good.
        let _ = state.room_manager.mark_peer_away(&room_code, peer_id).await;
        state.room_manager.broadcast_room_snapshot(&room_code).await;
        info!("client {peer_id} away (grace) in room {room_code_send}");

        let grace_state = Arc::clone(&state);
        let grace_code = room_code.clone();
        let slot = PlayerSlot(peer.session.player_id);
        tokio::spawn(async move {
            // Freeze the slot's engine immediately so it does not ghost-tick during
            // the grace window. A reclaim (ResumePlayer) unpauses it; grace expiry
            // removes the player outright.
            if let Ok(Some(actor_tx)) = grace_state.room_manager.actor_tx(&grace_code).await {
                let _ = actor_tx.send(RoomCommand::PauseSlot { slot }).await;
            }
            tokio::time::sleep(Duration::from_secs(RECONNECT_GRACE_SECS)).await;
            // Reclaimed during grace (peer.id rebound to a new connection) → no-op.
            if !grace_state
                .room_manager
                .peer_is_away(&grace_code, peer_id)
                .await
            {
                return;
            }
            if let Ok(Some(actor_tx)) = grace_state.room_manager.actor_tx(&grace_code).await {
                let _ = actor_tx.send(RoomCommand::PlayerLeave { slot }).await;
            }
            let remaining = grace_state
                .room_manager
                .remove_peer(&grace_code, peer_id)
                .await;
            grace_state.room_manager.leave_room(&grace_code).await;
            info!("client {peer_id} grace expired, removed from room {grace_code}");
            if !remaining.is_empty() {
                grace_state
                    .room_manager
                    .broadcast_snapshot(&grace_code, &remaining)
                    .await;
            }
        });
        return;
    }

    // Lobby / pre-game disconnect: immediate teardown (no engine to preserve).
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
        Ok(header) => {
            warn!(
                "dropping packet with unexpected protocol version {} from peer {peer_id}",
                header.version
            );
            return;
        }
        Err(e) => {
            warn!("failed to decode packet header from peer {peer_id}: {e}");
            return;
        }
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
                    state
                        .room_manager
                        .cancel_and_broadcast_countdown(room_code)
                        .await;
                }
                if all_ready {
                    let countdown_generation = state
                        .room_manager
                        .try_start_countdown(room_code)
                        .await
                        .ok()
                        .flatten();
                    if let Some(countdown_generation) = countdown_generation {
                        let state = Arc::clone(state);
                        let room_code_owned = room_code.to_string();
                        tokio::spawn(async move {
                            for remaining_secs in [3_u8, 2, 1, 0] {
                                let still_ready = state
                                    .room_manager
                                    .all_peers_ready(&room_code_owned)
                                    .await
                                    .unwrap_or(false);
                                let countdown_current = state
                                    .room_manager
                                    .countdown_generation_matches(
                                        &room_code_owned,
                                        countdown_generation,
                                    )
                                    .await
                                    .unwrap_or(false);
                                if !still_ready || !countdown_current {
                                    state
                                        .room_manager
                                        .cancel_and_broadcast_countdown(&room_code_owned)
                                        .await;
                                    return;
                                }
                                let countdown = PktStartCountdown {
                                    header: PacketHeader::new(PacketType::StartCountdown, 0),
                                    remaining_secs,
                                };
                                if let Ok(data) = serialize(&countdown) {
                                    let _ =
                                        state.room_manager.broadcast(&room_code_owned, data).await;
                                }
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                            if !state
                                .room_manager
                                .all_peers_ready(&room_code_owned)
                                .await
                                .unwrap_or(false)
                                || !state
                                    .room_manager
                                    .countdown_generation_matches(
                                        &room_code_owned,
                                        countdown_generation,
                                    )
                                    .await
                                    .unwrap_or(false)
                            {
                                state
                                    .room_manager
                                    .cancel_and_broadcast_countdown(&room_code_owned)
                                    .await;
                                return;
                            }
                            // 先取 pending channels 并判断是否已有运行中的 actor（第二局复用）。
                            let player_channels: Vec<PlayerChannel> = {
                                let mut pending = state.pending_inputs.lock().await;
                                pending.remove(&room_code_owned).unwrap_or_default()
                            };
                            let actor_exists = matches!(
                                state.room_manager.actor_tx(&room_code_owned).await,
                                Ok(Some(_))
                            );
                            // 无连接通道且无运行中 actor → 无法建局：取消倒计时回 lobby，
                            // 避免客户端收到 GameStart 却永久卡死（无 gamestate）。
                            if player_channels.is_empty() && !actor_exists {
                                state
                                    .room_manager
                                    .cancel_and_broadcast_countdown(&room_code_owned)
                                    .await;
                                state
                                    .room_manager
                                    .broadcast_room_snapshot(&room_code_owned)
                                    .await;
                                return;
                            }

                            let random_seed = rand::random::<u32>();
                            let game_start = PktGameStart {
                                header: PacketHeader::new(PacketType::GameStart, 0),
                                random_seed,
                            };
                            if let Ok(data) = serialize(&game_start) {
                                let _ = state.room_manager.broadcast(&room_code_owned, data).await;
                            }
                            info!("game starting in room {room_code_owned} (seed {random_seed})");

                            if player_channels.is_empty() {
                                // 第二局+：actor 仍在运行（GameOver 后未销毁），发 StartGame
                                // 重置 sim 并重新开局，无需重建 actor / 通道。
                                if let Ok(Some(actor_tx)) =
                                    state.room_manager.actor_tx(&room_code_owned).await
                                {
                                    let _ = actor_tx
                                        .send(RoomCommand::StartGame {
                                            seed: Seed(random_seed as i32),
                                        })
                                        .await;
                                }
                            } else {
                                // 第一局：新建 RoomActor。
                                let settings =
                                    state.room_manager.room_settings(&room_code_owned).await;
                                let mut actor = RoomActor::with_settings(
                                    room_code_owned.clone(),
                                    Seed(random_seed as i32),
                                    settings,
                                );
                                let room_peers = state
                                    .room_manager
                                    .room_peers(&room_code_owned)
                                    .await
                                    .unwrap_or_default();
                                for (player_id, input_rx, outbound_tx) in player_channels {
                                    let (conn_tx, _) = tokio::sync::mpsc::channel::<InputEvent>(64);
                                    let conn = make_player_connection(player_id, conn_tx);
                                    actor.add_player(
                                        PlayerSlot(player_id),
                                        input_rx,
                                        conn,
                                        outbound_tx,
                                    );
                                }
                                for peer in room_peers.iter().filter(|peer| peer.session.is_bot) {
                                    let _ = actor.add_bot(
                                        PlayerSlot(peer.session.player_id),
                                        peer.session.temperature,
                                    );
                                }
                                actor.set_room_mode(RoomMode::Playing);
                                let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
                                let (command_tx, command_rx) =
                                    tokio::sync::mpsc::channel::<RoomCommand>(64);
                                let _ = state
                                    .room_manager
                                    .store_actor_tx(&room_code_owned, command_tx)
                                    .await;
                                tokio::spawn(actor.run(cancel_rx, command_rx));
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
                                let _ = state.room_manager.cancel_countdown(&room_code_owned).await;
                                state
                                    .room_manager
                                    .broadcast_snapshot(&room_code_owned, &reset_peers)
                                    .await;
                            }
                        });
                    }
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
                    header: PacketHeader::new(PacketType::ChatMessage, peer.session.player_id),
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
        PacketType::AddBot => {
            let Ok(pkt) = deser::<PktAddBot>(&data) else {
                return;
            };
            let Ok(bot_peer) = state
                .room_manager
                .add_bot_peer(room_code, pkt.temperature)
                .await
            else {
                return;
            };
            info!(
                "bot added to room {room_code} as player {}",
                bot_peer.session.player_id
            );
            state.room_manager.broadcast_room_snapshot(room_code).await;
            if let Ok(Some(actor_tx)) = state.room_manager.actor_tx(room_code).await {
                let _ = actor_tx
                    .send(RoomCommand::AddBot {
                        slot: PlayerSlot(bot_peer.session.player_id),
                        temperature: pkt.temperature,
                    })
                    .await;
            }
        }
        // New protocol types (Replay, etc.) — forward to input_tx if RoomActor is active
        PacketType::RoomSettings => {
            let Ok(pkt) = deser::<PktRoomSettings>(&data) else {
                return;
            };
            if !state.room_manager.is_host(room_code, peer_id).await {
                warn!("non-host peer {peer_id} attempted RoomSettings; rejected");
                return;
            }
            let rules = RoomRules {
                start_level: u32::from(pkt.start_level).clamp(1, 15),
                hold_enabled: pkt.allow_hold,
                initial_garbage_lines: pkt.initial_garbage_lines.min(MAX_INITIAL_GARBAGE_LINES),
            };
            let garbage_delay_ticks = u16::from(pkt.garbage_delay_secs)
                .saturating_mul(60)
                .min(MAX_GARBAGE_DELAY_TICKS);
            let _ = state
                .room_manager
                .set_room_settings(
                    room_code,
                    RelaySettings {
                        rules,
                        garbage_delay_ticks,
                    },
                )
                .await;
            // Re-broadcast so every client syncs the displayed rules.
            let _ = state.room_manager.broadcast(room_code, data).await;
        }
        PacketType::KickPlayer => {
            let Ok(pkt) = deser::<PktKickPlayer>(&data) else {
                return;
            };
            // Only the host may kick, and never kick the host (self).
            if !state.room_manager.is_host(room_code, peer_id).await {
                warn!("non-host peer {peer_id} attempted KickPlayer; rejected");
                return;
            }
            let Ok(target) = state
                .room_manager
                .peer_by_player_id(room_code, pkt.target_player_id)
                .await
            else {
                return;
            };
            if target.id == peer_id {
                return;
            }
            // Notify the kicked client (still subscribed) so it returns home, then
            // tear down its slot in the sim (if any) and remove the peer.
            let _ = state.room_manager.broadcast(room_code, data).await;
            if let Ok(Some(actor_tx)) = state.room_manager.actor_tx(room_code).await {
                let _ = actor_tx
                    .send(RoomCommand::PlayerLeave {
                        slot: PlayerSlot(target.session.player_id),
                    })
                    .await;
            }
            state.room_manager.remove_peer(room_code, target.id).await;
            info!(
                "host {peer_id} kicked player {} from room {room_code}",
                target.session.player_id
            );
            state.room_manager.broadcast_room_snapshot(room_code).await;
        }
        PacketType::RemoveBot => {
            let Ok(pkt) = deser::<PktRemoveBot>(&data) else {
                return;
            };
            if !state.room_manager.is_host(room_code, peer_id).await {
                warn!("non-host peer {peer_id} attempted RemoveBot; rejected");
                return;
            }
            let Ok(target) = state
                .room_manager
                .peer_by_player_id(room_code, pkt.target_player_id)
                .await
            else {
                return;
            };
            if !target.session.is_bot {
                return;
            }
            if let Ok(Some(actor_tx)) = state.room_manager.actor_tx(room_code).await {
                let _ = actor_tx
                    .send(RoomCommand::PlayerLeave {
                        slot: PlayerSlot(target.session.player_id),
                    })
                    .await;
            }
            state.room_manager.remove_peer(room_code, target.id).await;
            info!(
                "host {peer_id} removed bot {} from room {room_code}",
                target.session.player_id
            );
            state.room_manager.broadcast_room_snapshot(room_code).await;
        }
        // New protocol types (Replay, etc.) — forward to input_tx if RoomActor is active
        PacketType::Replay => {
            if let Ok(pkt) = deser::<PktReplay>(&data)
                && replay_packet_is_valid(&pkt)
            {
                debug!(
                    "replay input from peer {peer_id}: {} events @ start_tick {}",
                    pkt.events.len(),
                    pkt.start_tick.0
                );
                for ev in pkt.events {
                    let _ = input_tx.try_send(ev);
                }
            }
        }
        PacketType::Resume => {
            // Resume is authenticated and acted on during the connection handshake
            // (see handle_socket). A mid-session Resume is a no-op; we only check
            // the token shape against the peer's own token and never act on a
            // mismatch — the slot is never reassigned from here.
            let Ok(pkt) = deser::<PktResume>(&data) else {
                return;
            };
            if let Ok(peer) = state.room_manager.peer_by_id(room_code, peer_id).await {
                if pkt.resume_token == peer.session.resume_token {
                    debug!("mid-session resume from peer {peer_id} (matching token, no-op)");
                } else {
                    warn!("ignoring mid-session resume with non-matching token for peer {peer_id}");
                }
            }
        }
        PacketType::Reconnect => {
            let Ok(pkt) = deser::<PktReconnect>(&data) else {
                return;
            };
            let Ok(peer) = state.room_manager.peer_by_id(room_code, peer_id).await else {
                return;
            };
            if let Ok(Some(actor_tx)) = state.room_manager.actor_tx(room_code).await {
                let _ = actor_tx
                    .send(RoomCommand::Reconnect {
                        slot: PlayerSlot(peer.session.player_id),
                        client_hashes: pkt.client_hashes,
                    })
                    .await;
            }
        }
        PacketType::Connect => {
            warn!("unexpected mid-session Connect packet from peer {peer_id}");
        }
        _ => {}
    }
}

fn make_player_connection(
    player_id: u8,
    tx: tokio::sync::mpsc::Sender<InputEvent>,
) -> crate::player_conn::PlayerConnection<crate::player_conn::Online> {
    crate::player_conn::PlayerConnection::<crate::player_conn::Online>::new(
        PlayerSlot(player_id),
        tx,
        format!("Player {}", player_id + 1),
    )
}

fn binary_packet_is_replay(data: &[u8]) -> bool {
    deser::<PacketHeader>(data).is_ok_and(|header| {
        header.version == PROTOCOL_VERSION && header.packet_type == PacketType::Replay
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_protocol::newtypes::{KeyAction, TickNumber};
    use tetris_protocol::protocol::PktReconnectAck;

    fn make_replay(player_id: u8, events: Vec<InputEvent>) -> PktReplay {
        PktReplay {
            header: PacketHeader::new(PacketType::Replay, player_id),
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

    #[tokio::test]
    async fn reconnect_routes_to_sim() {
        let manager = Arc::new(RoomManager::new(4));
        manager.get_or_create_room("ABCD").await.unwrap();
        let peer_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", peer_id).await.unwrap();
        let state = Arc::new(AppState {
            room_manager: Arc::clone(&manager),
            pending_inputs: Arc::new(Mutex::new(HashMap::new())),
        });

        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn_tx, input_rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = make_player_connection(0, conn_tx);
        let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        actor.add_player(PlayerSlot(0), input_rx, conn, out_tx);
        for _ in 0..100 {
            actor.run_one_tick();
        }
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        let (command_tx, command_rx) = tokio::sync::mpsc::channel::<RoomCommand>(64);
        manager.store_actor_tx("ABCD", command_tx).await.unwrap();
        let actor_task = tokio::spawn(actor.run(cancel_rx, command_rx));

        let pkt = PktReconnect {
            header: PacketHeader::new(PacketType::Reconnect, 0),
            last_good_tick: TickNumber(100),
            client_hashes: vec![(TickNumber(100), 0xDEAD_BEEF)],
        };
        let data = serialize(&pkt).unwrap();
        let (input_tx, _input_rx) = tokio::sync::mpsc::channel::<InputEvent>(4);

        handle_binary_message(&state, "ABCD", peer_id, &input_tx, data).await;

        let ack = tokio::time::timeout(Duration::from_millis(100), async {
            loop {
                let bytes = out_rx.recv().await.unwrap();
                let header: PacketHeader = deser(&bytes).unwrap();
                if header.packet_type == PacketType::ReconnectAck {
                    break deser::<PktReconnectAck>(&bytes).unwrap();
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(ack.divergence_tick, TickNumber(100));

        let _ = cancel_tx.send(());
        let _ = actor_task.await;
    }

    #[test]
    fn replay_packet_rejects_invalid_tick_window() {
        let pkt = make_replay(0, vec![make_event(131)]);

        assert!(!replay_packet_is_valid(&pkt));
    }

    #[test]
    fn replay_rate_limit_exempts_replay_packet_from_message_drop() {
        let pkt = make_replay(0, vec![make_event(10)]);
        let data = serialize(&pkt).unwrap();

        assert!(binary_packet_is_replay(&data));
    }

    #[tokio::test]
    async fn countdown_cancel_invalidates_started_generation() {
        let manager = RoomManager::new(4);
        manager.get_or_create_room("ABCD").await.unwrap();
        let first_id = RoomManager::alloc_peer_id();
        let second_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", first_id).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", second_id).await.unwrap();
        manager
            .set_peer_ready("ABCD", first_id, true)
            .await
            .unwrap();
        manager
            .set_peer_ready("ABCD", second_id, true)
            .await
            .unwrap();
        let generation = manager.try_start_countdown("ABCD").await.unwrap().unwrap();

        manager
            .set_peer_ready("ABCD", second_id, false)
            .await
            .unwrap();

        assert!(
            !manager
                .countdown_generation_matches("ABCD", generation)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn kick_player_rejected_for_non_host() {
        use tetris_protocol::protocol::PktKickPlayer;
        let manager = Arc::new(RoomManager::new(4));
        manager.get_or_create_room("ABCD").await.unwrap();
        let host_id = RoomManager::alloc_peer_id();
        let other_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        let host_peers = manager.add_peer("ABCD", host_id).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", other_id).await.unwrap();
        let host_slot = host_peers[0].session.player_id;
        let state = Arc::new(AppState {
            room_manager: Arc::clone(&manager),
            pending_inputs: Arc::new(Mutex::new(HashMap::new())),
        });

        // Non-host (other_id) tries to kick the host slot → must be rejected.
        let pkt = PktKickPlayer {
            header: PacketHeader::new(PacketType::KickPlayer, 1),
            target_player_id: host_slot,
        };
        let data = serialize(&pkt).unwrap();
        let (input_tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(4);

        handle_binary_message(&state, "ABCD", other_id, &input_tx, data).await;

        assert_eq!(manager.room_peers("ABCD").await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn kick_player_removes_target_for_host() {
        use tetris_protocol::protocol::PktKickPlayer;
        let manager = Arc::new(RoomManager::new(4));
        manager.get_or_create_room("ABCD").await.unwrap();
        let host_id = RoomManager::alloc_peer_id();
        let other_id = RoomManager::alloc_peer_id();
        manager.join_room("ABCD").await.unwrap();
        manager.add_peer("ABCD", host_id).await.unwrap();
        manager.join_room("ABCD").await.unwrap();
        let other_peers = manager.add_peer("ABCD", other_id).await.unwrap();
        let target_slot = other_peers
            .iter()
            .find(|p| p.id == other_id)
            .unwrap()
            .session
            .player_id;
        let state = Arc::new(AppState {
            room_manager: Arc::clone(&manager),
            pending_inputs: Arc::new(Mutex::new(HashMap::new())),
        });

        // Host kicks the other player → removed.
        let pkt = PktKickPlayer {
            header: PacketHeader::new(PacketType::KickPlayer, 0),
            target_player_id: target_slot,
        };
        let data = serialize(&pkt).unwrap();
        let (input_tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(4);

        handle_binary_message(&state, "ABCD", host_id, &input_tx, data).await;

        let peers = manager.room_peers("ABCD").await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].id, host_id);
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

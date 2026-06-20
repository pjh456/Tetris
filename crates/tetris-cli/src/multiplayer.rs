use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use futures_util::{SinkExt, StreamExt};
use tetris_core::engine::{Action, Engine};
use tetris_infer::MlpPolicy;
use tetris_net::bot::AiBot;
use tetris_net::host_adapter::RenetHostAdapter;
use tetris_net::network_manager::{MODEL_B_CHANNEL, NetworkManager};
use tetris_protocol::newtypes::{KeyAction, PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::{
    InputEvent, PROTOCOL_VERSION, PacketHeader, PacketType, PktIncomingGarbage, PktReplay,
    PktServerAccept, PktServerReplay, PktStateHash, PktStateSnapshot,
};

const FLUSH_TICK_INTERVAL: u64 = 30;
const MAX_BATCH_EVENTS: usize = 64;
const MAX_PLAYERS: u8 = 4;
const DEFAULT_WEIGHTS_PATH: &str = "models/weights.json";
const BOT_TICK_MS: u64 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiplayerMode {
    JoinRelay { url: String, room_code: String },
    HostP2p { bind_addr: SocketAddr },
    JoinP2p { addr: SocketAddr },
}

impl MultiplayerMode {
    pub fn join_relay(url: impl Into<String>, room_code: impl Into<String>) -> Self {
        Self::JoinRelay {
            url: url.into(),
            room_code: room_code.into(),
        }
    }

    pub fn host_p2p(bind_addr: &str) -> Result<Self, crate::error::CliError> {
        Ok(Self::HostP2p {
            bind_addr: parse_socket_addr(bind_addr)?,
        })
    }

    pub fn join_p2p(addr: &str) -> Result<Self, crate::error::CliError> {
        Ok(Self::JoinP2p {
            addr: parse_socket_addr(addr)?,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Online,
    Slow,
    Reconnecting,
    Disconnected,
    Resyncing,
}

impl ConnectionStatus {
    pub fn label(self) -> &'static str {
        match self {
            ConnectionStatus::Online => "ONLINE",
            ConnectionStatus::Slow => "SLOW",
            ConnectionStatus::Reconnecting => "RECONNECTING",
            ConnectionStatus::Disconnected => "DISCONNECTED",
            ConnectionStatus::Resyncing => "RESYNCING",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpponentView {
    pub engine: Engine<10, 20>,
    pub name: String,
    pub status: ConnectionStatus,
    pub incoming_garbage: u8,
    pub alive: bool,
    pub spectating: bool,
}

impl OpponentView {
    pub fn new(name: impl Into<String>, seed: u32) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(seed);
        Self {
            engine,
            name: name.into(),
            status: ConnectionStatus::Online,
            incoming_garbage: 0,
            alive: true,
            spectating: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultiplayerSession {
    pub mode: MultiplayerMode,
    pub player_id: u8,
    pub status: ConnectionStatus,
    pub input_buffer: CliInputBuffer,
    pub outbox: Vec<Vec<u8>>,
    pub last_server_hash: Option<(TickNumber, u32)>,
}

impl MultiplayerSession {
    pub fn new(mode: MultiplayerMode, player_id: u8) -> Self {
        Self {
            mode,
            player_id,
            status: ConnectionStatus::Online,
            input_buffer: CliInputBuffer::new(),
            outbox: Vec::new(),
            last_server_hash: None,
        }
    }

    pub fn push_packet(&mut self, packet: Vec<u8>) {
        if !packet.is_empty() {
            self.outbox.push(packet);
        }
    }

    pub fn mode_label(&self) -> String {
        match &self.mode {
            MultiplayerMode::JoinRelay { url, room_code } => {
                format!("relay {url} room {room_code}")
            }
            MultiplayerMode::HostP2p { bind_addr } => format!("p2p host {bind_addr}"),
            MultiplayerMode::JoinP2p { addr } => format!("p2p join {addr}"),
        }
    }
}

pub struct CliNetworkRuntime {
    key: String,
    kind: CliNetworkKind,
    last_tick: Instant,
}

enum CliNetworkKind {
    Relay {
        outbound_tx: Sender<Vec<u8>>,
        inbound_rx: Receiver<Vec<u8>>,
    },
    P2pHost {
        net: NetworkManager,
        adapter: RenetHostAdapter,
    },
    P2pClient {
        net: NetworkManager,
    },
}

impl CliNetworkRuntime {
    pub fn connect(mode: &MultiplayerMode) -> Result<Self, crate::error::CliError> {
        let key = Self::mode_key(mode);
        let kind = match mode {
            MultiplayerMode::JoinRelay { url, room_code } => {
                let (outbound_tx, outbound_rx) = mpsc::channel();
                let (inbound_tx, inbound_rx) = mpsc::channel();
                spawn_relay_thread(relay_room_url(url, room_code), outbound_rx, inbound_tx);
                CliNetworkKind::Relay {
                    outbound_tx,
                    inbound_rx,
                }
            }
            MultiplayerMode::HostP2p { bind_addr } => {
                let mut net = NetworkManager::new();
                net.start_server(&bind_addr.ip().to_string(), bind_addr.port(), MAX_PLAYERS)
                    .map_err(|e| crate::error::CliError::Network(e.to_string()))?;
                let mut adapter = RenetHostAdapter::new(Seed(42), MAX_PLAYERS);
                adapter.start_playing();
                CliNetworkKind::P2pHost { net, adapter }
            }
            MultiplayerMode::JoinP2p { addr } => {
                let mut net = NetworkManager::new();
                net.connect_to_server(&addr.ip().to_string(), addr.port())
                    .map_err(|e| crate::error::CliError::Network(e.to_string()))?;
                CliNetworkKind::P2pClient { net }
            }
        };
        Ok(Self {
            key,
            kind,
            last_tick: Instant::now(),
        })
    }

    pub fn mode_key(mode: &MultiplayerMode) -> String {
        match mode {
            MultiplayerMode::JoinRelay { url, room_code } => format!("relay:{url}:{room_code}"),
            MultiplayerMode::HostP2p { bind_addr } => format!("host:{bind_addr}"),
            MultiplayerMode::JoinP2p { addr } => format!("join:{addr}"),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn p2p_server_addr(&self) -> Option<SocketAddr> {
        match &self.kind {
            CliNetworkKind::P2pHost { net, .. } => net.server_addr(),
            _ => None,
        }
    }

    pub fn pump(&mut self, session: &mut MultiplayerSession) -> Vec<Vec<u8>> {
        let outbound = std::mem::take(&mut session.outbox);
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;

        match &mut self.kind {
            CliNetworkKind::Relay {
                outbound_tx,
                inbound_rx,
            } => {
                for packet in outbound {
                    let _ = outbound_tx.send(packet);
                }
                drain_inbound(inbound_rx)
            }
            CliNetworkKind::P2pHost { net, adapter } => {
                for packet in outbound {
                    if let Ok(replay) = bincode::deserialize::<PktReplay>(&packet) {
                        for event in replay.events {
                            adapter
                                .sim
                                .enqueue_input(PlayerSlot(session.player_id), event);
                        }
                    }
                }
                if adapter.tick(net, delta).is_err() {
                    session.status = ConnectionStatus::Disconnected;
                }
                Vec::new()
            }
            CliNetworkKind::P2pClient { net } => {
                for packet in outbound {
                    net.broadcast(MODEL_B_CHANNEL, packet);
                }
                if net.tick(delta).is_err() {
                    session.status = ConnectionStatus::Disconnected;
                }
                net.receive_messages(MODEL_B_CHANNEL)
            }
        }
    }
}

fn drain_inbound(inbound_rx: &Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(packet) = inbound_rx.try_recv() {
        packets.push(packet);
    }
    packets
}

fn relay_room_url(url: &str, room_code: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.ends_with(room_code) {
        trimmed.to_string()
    } else {
        format!("{trimmed}/{room_code}")
    }
}

fn spawn_relay_thread(url: String, outbound_rx: Receiver<Vec<u8>>, inbound_tx: Sender<Vec<u8>>) {
    thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async move {
            let Ok((socket, _)) = tokio_tungstenite::connect_async(&url).await else {
                return;
            };
            let (mut writer, mut reader) = socket.split();
            loop {
                while let Ok(packet) = outbound_rx.try_recv() {
                    if writer
                        .send(tokio_tungstenite::tungstenite::Message::Binary(
                            packet.into(),
                        ))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }

                tokio::select! {
                    Some(message) = reader.next() => {
                        match message {
                            Ok(tokio_tungstenite::tungstenite::Message::Binary(data)) => {
                                let _ = inbound_tx.send(data.to_vec());
                            }
                            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => return,
                            _ => {}
                        }
                    }
                    () = tokio::time::sleep(Duration::from_millis(5)) => {}
                }
            }
        });
    });
}

pub struct AiOpponentHandle {
    join: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    sent_replays: Arc<AtomicUsize>,
}

impl AiOpponentHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    pub fn sent_replay_count(&self) -> usize {
        self.sent_replays.load(Ordering::SeqCst)
    }
}

impl Drop for AiOpponentHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn spawn_ai_opponent(
    mode: &MultiplayerMode,
    temperature: f32,
) -> Result<AiOpponentHandle, crate::error::CliError> {
    let policy = load_ai_opponent_policy()?;
    let connect_mode = bot_connect_mode(mode)?;
    let seed = default_seed_for_mode(mode);
    spawn_ai_opponent_with_policy(connect_mode, seed, policy, temperature)
}

pub fn spawn_ai_opponent_for_runtime(
    mode: &MultiplayerMode,
    runtime: &CliNetworkRuntime,
    temperature: f32,
) -> Result<AiOpponentHandle, crate::error::CliError> {
    let policy = load_ai_opponent_policy()?;
    spawn_ai_opponent_for_runtime_with_policy(mode, runtime, policy, temperature)
}

pub fn validate_ai_opponent_weights() -> Result<(), crate::error::CliError> {
    load_ai_opponent_policy().map(|_| ())
}

fn load_ai_opponent_policy() -> Result<MlpPolicy, crate::error::CliError> {
    let path = weights_path();
    let weights = std::fs::read(&path)
        .map_err(|e| crate::error::CliError::Network(format!("failed to read {path}: {e}")))?;
    MlpPolicy::load_from_slice(&weights)
        .map_err(|e| crate::error::CliError::Network(format!("failed to load {path}: {e}")))
}

fn spawn_ai_opponent_for_runtime_with_policy(
    mode: &MultiplayerMode,
    runtime: &CliNetworkRuntime,
    policy: MlpPolicy,
    temperature: f32,
) -> Result<AiOpponentHandle, crate::error::CliError> {
    let connect_mode = bot_connect_mode_for_runtime(mode, runtime)?;
    let seed = default_seed_for_mode(mode);
    spawn_ai_opponent_with_policy(connect_mode, seed, policy, temperature)
}

fn weights_path() -> String {
    std::env::var("TETRIS_WEIGHTS_PATH").unwrap_or_else(|_| DEFAULT_WEIGHTS_PATH.into())
}

fn spawn_ai_opponent_with_policy(
    connect_mode: MultiplayerMode,
    seed: Seed,
    policy: MlpPolicy,
    temperature: f32,
) -> Result<AiOpponentHandle, crate::error::CliError> {
    let mut runtime = CliNetworkRuntime::connect(&connect_mode)?;
    let mut session = MultiplayerSession::new(connect_mode, 0);
    let mut bot = AiBot::new(policy, seed.0 as u32, temperature);
    let stop = Arc::new(AtomicBool::new(false));
    let sent_replays = Arc::new(AtomicUsize::new(0));
    let thread_stop = Arc::clone(&stop);
    let thread_sent_replays = Arc::clone(&sent_replays);

    let join = thread::spawn(move || {
        while !thread_stop.load(Ordering::SeqCst) {
            if let Some(replay) = bot.next_replay(session.player_id)
                && let Ok(packet) = bincode::serialize(&replay)
            {
                session.push_packet(packet);
                thread_sent_replays.fetch_add(1, Ordering::SeqCst);
            }

            for packet in runtime.pump(&mut session) {
                observe_ai_packet(&mut bot, &mut session, &packet);
            }
            thread::sleep(Duration::from_millis(BOT_TICK_MS));
        }
    });

    Ok(AiOpponentHandle {
        join: Some(join),
        stop,
        sent_replays,
    })
}

fn bot_connect_mode(mode: &MultiplayerMode) -> Result<MultiplayerMode, crate::error::CliError> {
    match mode {
        MultiplayerMode::HostP2p { bind_addr } => {
            if bind_addr.port() == 0 {
                return Err(crate::error::CliError::Network(
                    "cannot spawn AI opponent before p2p host has a bound port".into(),
                ));
            }
            Ok(MultiplayerMode::JoinP2p { addr: *bind_addr })
        }
        MultiplayerMode::JoinP2p { .. } | MultiplayerMode::JoinRelay { .. } => Ok(mode.clone()),
    }
}

fn bot_connect_mode_for_runtime(
    mode: &MultiplayerMode,
    runtime: &CliNetworkRuntime,
) -> Result<MultiplayerMode, crate::error::CliError> {
    match mode {
        MultiplayerMode::HostP2p { .. } => {
            let Some(addr) = runtime.p2p_server_addr() else {
                return Err(crate::error::CliError::Network(
                    "p2p host runtime has no bound server address".into(),
                ));
            };
            Ok(MultiplayerMode::JoinP2p { addr })
        }
        MultiplayerMode::JoinP2p { .. } | MultiplayerMode::JoinRelay { .. } => Ok(mode.clone()),
    }
}

fn observe_ai_packet(bot: &mut AiBot, session: &mut MultiplayerSession, data: &[u8]) {
    let Ok(header) = bincode::deserialize::<PacketHeader>(data) else {
        return;
    };
    match header.packet_type {
        PacketType::StateSnapshot => {
            if let Ok(pkt) = bincode::deserialize::<PktStateSnapshot>(data) {
                let mut engine = Engine::<10, 20>::new();
                apply_state_snapshot(&mut engine, &pkt);
                bot.observe_engine(engine);
            }
        }
        PacketType::ServerAccept => {
            if let Ok(pkt) = bincode::deserialize::<PktServerAccept>(data) {
                session.player_id = pkt.assigned_player_id;
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
pub struct CliInputBuffer {
    current_tick: TickNumber,
    last_flush_tick: TickNumber,
    pending: Vec<InputEvent>,
}

impl CliInputBuffer {
    pub fn new() -> Self {
        Self {
            current_tick: TickNumber(0),
            last_flush_tick: TickNumber(0),
            pending: Vec::new(),
        }
    }

    pub fn advance_tick(&mut self) {
        self.current_tick.0 = self.current_tick.0.saturating_add(1);
    }

    pub fn push_key(&mut self, key: KeyCode, pressed: bool, subframe: f32) -> Option<InputEvent> {
        let key = key_action_for_keycode(key)?;
        let event = InputEvent {
            key,
            pressed,
            tick: self.current_tick,
            subframe,
        };
        self.pending.push(event.clone());
        Some(event)
    }

    pub fn should_flush(&self) -> bool {
        !self.pending.is_empty()
            && (self.current_tick.0.saturating_sub(self.last_flush_tick.0) >= FLUSH_TICK_INTERVAL
                || self.pending.len() >= MAX_BATCH_EVENTS)
    }

    pub fn flush_replay(&mut self, player_id: u8) -> Option<Vec<u8>> {
        if self.pending.is_empty() {
            return None;
        }
        let events = std::mem::take(&mut self.pending);
        let start_tick = events.first().map_or(self.current_tick, |event| event.tick);
        self.last_flush_tick = self.current_tick;
        let pkt = PktReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Replay,
                player_id,
            },
            events,
            start_tick,
        };
        bincode::serialize(&pkt).ok()
    }
}

impl Default for CliInputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

pub fn key_action_for_keycode(key: KeyCode) -> Option<KeyAction> {
    match key {
        KeyCode::Left | KeyCode::Char('a') => Some(KeyAction::KeyLeft),
        KeyCode::Right | KeyCode::Char('d') => Some(KeyAction::KeyRight),
        KeyCode::Down | KeyCode::Char('s') => Some(KeyAction::KeySoftDrop),
        KeyCode::Char(' ') => Some(KeyAction::KeyHardDrop),
        KeyCode::Up | KeyCode::Char('w' | 'x') => Some(KeyAction::KeyRotateCW),
        KeyCode::Char('z') => Some(KeyAction::KeyRotateCCW),
        KeyCode::Tab => Some(KeyAction::KeyHold),
        _ => None,
    }
}

pub fn action_for_key_action(key: KeyAction) -> Action {
    match key {
        KeyAction::KeyLeft => Action::MoveLeft,
        KeyAction::KeyRight => Action::MoveRight,
        KeyAction::KeySoftDrop => Action::SoftDrop,
        KeyAction::KeyHardDrop => Action::HardDrop,
        KeyAction::KeyRotateCW => Action::RotateCW,
        KeyAction::KeyRotateCCW => Action::RotateCCW,
        KeyAction::KeyHold => Action::Hold,
    }
}

pub fn apply_input_prediction(engine: &mut Engine<10, 20>, event: &InputEvent) {
    if event.pressed {
        engine.handle_action(action_for_key_action(event.key));
    }
}

pub fn apply_state_snapshot(engine: &mut Engine<10, 20>, snapshot: &PktStateSnapshot) {
    engine.reset(snapshot.seed.0 as u32);
    for (idx, row) in snapshot.board_rows.iter().copied().enumerate().take(20) {
        engine.state.board.rows[idx] = row;
    }
    engine.state.piece = snapshot.piece;
    engine.state.rot = snapshot.rot;
    engine.state.x = snapshot.x;
    engine.state.y = snapshot.y;
    engine.state.hold = snapshot.hold;
    engine.state.hold_used = snapshot.hold_used;
    engine.has_hold = true;
    engine.state.next = snapshot.next;
    engine.state.rng = snapshot.rng_state;
    engine.state.combo = snapshot.combo;
    engine.state.b2b = snapshot.b2b;
    engine.state.pending_garbage = snapshot.pending_garbage;
}

pub fn apply_server_replay(
    local_player_id: u8,
    local_engine: &mut Engine<10, 20>,
    opponents: &mut Vec<OpponentView>,
    replay: &PktServerReplay,
) {
    let target_engine = if replay.source_player == PlayerSlot(local_player_id) {
        local_engine
    } else {
        let idx = usize::from(replay.source_player.0);
        ensure_opponent(opponents, idx);
        &mut opponents[idx].engine
    };

    for event in &replay.events {
        apply_input_prediction(target_engine, event);
    }

    if replay.source_player != PlayerSlot(local_player_id) {
        let idx = usize::from(replay.source_player.0);
        opponents[idx].incoming_garbage = replay.ige_garbage_lines;
    }
}

pub fn apply_authority_packet(
    local_player_id: u8,
    local_engine: &mut Engine<10, 20>,
    opponents: &mut Vec<OpponentView>,
    session: &mut MultiplayerSession,
    data: &[u8],
) -> bool {
    let Ok(header) = bincode::deserialize::<PacketHeader>(data) else {
        return false;
    };
    match header.packet_type {
        PacketType::ServerReplay => {
            let Ok(pkt) = bincode::deserialize::<PktServerReplay>(data) else {
                return false;
            };
            apply_server_replay(local_player_id, local_engine, opponents, &pkt);
            true
        }
        PacketType::StateSnapshot => {
            let Ok(pkt) = bincode::deserialize::<PktStateSnapshot>(data) else {
                return false;
            };
            apply_state_snapshot(local_engine, &pkt);
            session.status = ConnectionStatus::Online;
            true
        }
        PacketType::StateHash => {
            let Ok(pkt) = bincode::deserialize::<PktStateHash>(data) else {
                return false;
            };
            session.last_server_hash = Some((pkt.tick, pkt.hash));
            if local_engine.state_hash() != pkt.hash {
                session.status = ConnectionStatus::Resyncing;
            }
            true
        }
        PacketType::IncomingGarbage => {
            let Ok(pkt) = bincode::deserialize::<PktIncomingGarbage>(data) else {
                return false;
            };
            local_engine.add_pending_garbage(pkt.incoming_lines, 0, 0);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
pub fn make_state_hash_packet(player_id: u8, tick: TickNumber, hash: u32) -> Vec<u8> {
    let pkt = PktStateHash {
        header: PacketHeader {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::StateHash,
            player_id,
        },
        tick,
        hash,
    };
    bincode::serialize(&pkt).unwrap_or_default()
}

pub fn default_seed_for_mode(mode: &MultiplayerMode) -> Seed {
    match mode {
        MultiplayerMode::JoinRelay { .. } => Seed(0),
        MultiplayerMode::HostP2p { .. } => Seed(42),
        MultiplayerMode::JoinP2p { .. } => Seed(0),
    }
}

fn ensure_opponent(opponents: &mut Vec<OpponentView>, idx: usize) {
    while opponents.len() <= idx {
        let player_id = opponents.len() as u8;
        opponents.push(OpponentView::new(format!("P{player_id}"), 0));
    }
}

fn parse_socket_addr(value: &str) -> Result<SocketAddr, crate::error::CliError> {
    value.parse().map_err(|e| {
        crate::error::CliError::Network(format!("invalid socket address {value}: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::rl;
    use tetris_infer::Layer;
    use tetris_protocol::protocol::{PktServerReplay, PktStateSnapshot};

    fn zero_policy() -> MlpPolicy {
        MlpPolicy::new(
            rl::OBS_DIM,
            rl::ACTION_SPACE_SIZE,
            vec![Layer {
                weight: vec![vec![0.0; rl::OBS_DIM]; rl::ACTION_SPACE_SIZE],
                bias: vec![0.0; rl::ACTION_SPACE_SIZE],
            }],
        )
    }

    #[test]
    fn relay_client_parse() {
        let mode = MultiplayerMode::join_relay("ws://127.0.0.1:3000/ws", "ABCD");
        assert_eq!(
            mode,
            MultiplayerMode::JoinRelay {
                url: "ws://127.0.0.1:3000/ws".into(),
                room_code: "ABCD".into(),
            }
        );
    }

    #[test]
    fn renet_mode_selection() {
        assert!(matches!(
            MultiplayerMode::host_p2p("127.0.0.1:0").unwrap(),
            MultiplayerMode::HostP2p { .. }
        ));
        assert!(matches!(
            MultiplayerMode::join_p2p("127.0.0.1:5000").unwrap(),
            MultiplayerMode::JoinP2p { .. }
        ));
    }

    #[test]
    fn cli_input_batching() {
        let mut buffer = CliInputBuffer::new();
        buffer.push_key(KeyCode::Left, true, 0.0).unwrap();
        assert!(!buffer.should_flush());
        for _ in 0..30 {
            buffer.advance_tick();
        }
        assert!(buffer.should_flush());

        let packet = buffer.flush_replay(2).unwrap();
        let replay: PktReplay = bincode::deserialize(&packet).unwrap();
        assert_eq!(replay.header.player_id, 2);
        assert_eq!(replay.start_tick, TickNumber(0));
        assert_eq!(replay.events.len(), 1);
    }

    #[test]
    fn cli_reconcile() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let mut snapshot = PktStateSnapshot {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSnapshot,
                player_id: 0,
            },
            tick: TickNumber(7),
            board_rows: vec![0; 20],
            piece: tetris_core::types::Piece::T,
            rot: tetris_core::types::Rot::R0,
            x: 4,
            y: 5,
            hold: tetris_core::types::Piece::I,
            hold_used: false,
            next: [tetris_core::types::Piece::I; 5],
            rng_state: 42,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            seed: Seed(42),
        };
        snapshot.board_rows[19] = 0x3ff;
        apply_state_snapshot(&mut engine, &snapshot);

        assert_eq!(engine.state.y, 5);
        assert_eq!(engine.state.board.rows[19], 0x3ff);
    }

    #[test]
    fn server_replay_updates_remote_engine() {
        let mut local = Engine::<10, 20>::new();
        local.reset(42);
        let mut opponents = vec![OpponentView::new("P0", 42), OpponentView::new("P1", 42)];
        let before_hash = opponents[1].engine.state_hash();
        let replay = PktServerReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ServerReplay,
                player_id: 0,
            },
            source_player: PlayerSlot(1),
            events: vec![InputEvent {
                key: KeyAction::KeyHardDrop,
                pressed: true,
                tick: TickNumber(0),
                subframe: 0.0,
            }],
            ige_garbage_lines: 2,
            ige_hole_x: 4,
        };

        apply_server_replay(0, &mut local, &mut opponents, &replay);

        assert_ne!(opponents[1].engine.state_hash(), before_hash);
        assert_eq!(opponents[1].incoming_garbage, 2);
    }

    #[test]
    fn state_hash_packet_round_trip() {
        let bytes = make_state_hash_packet(2, TickNumber(5), 0xdead_beef);
        let pkt: PktStateHash = bincode::deserialize(&bytes).unwrap();
        assert_eq!(pkt.hash, 0xdead_beef);
        assert_eq!(pkt.tick, TickNumber(5));
    }

    #[test]
    fn spawn_ai_opponent_sends_replay_after_spawn() {
        let host_mode = MultiplayerMode::host_p2p("127.0.0.1:0").unwrap();
        let mut host_runtime = CliNetworkRuntime::connect(&host_mode).unwrap();
        let host_addr = host_runtime.p2p_server_addr().unwrap();
        let bot_mode = MultiplayerMode::JoinP2p { addr: host_addr };
        let handle = spawn_ai_opponent_with_policy(bot_mode, Seed(42), zero_policy(), 0.0).unwrap();
        let mut host_session = MultiplayerSession::new(host_mode, 0);

        for _ in 0..64 {
            let _ = host_runtime.pump(&mut host_session);
            if handle.sent_replay_count() > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(BOT_TICK_MS));
        }

        assert!(handle.sent_replay_count() > 0);
        handle.stop();
    }
}

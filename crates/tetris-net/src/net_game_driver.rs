use std::collections::HashMap;

use bincode::Options;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};
use tetris_core::engine::{Action, Engine};

use crate::error::NetError;
use crate::network_manager::NetworkManager;
use crate::protocol::*;

const MAX_PACKET_BYTES: u64 = 65536;

fn deser<'de, T: serde::Deserialize<'de>>(data: &'de [u8]) -> Result<T, bincode::Error> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(MAX_PACKET_BYTES)
        .deserialize::<T>(data)
}

new_key_type! { pub struct PlayerKey; }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RoomMode {
    #[default]
    Lobby,
    Countdown(u8),
    Playing,
    GameOver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSettings {
    pub max_players: u8,
    pub start_level: u8,
    pub attack_mult: f32,
    pub garbage_delay_secs: u8,
    pub allow_hold: bool,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            max_players: 4,
            start_level: 1,
            attack_mult: 1.0,
            garbage_delay_secs: 1,
            allow_hold: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub player_id: u8,
    pub name: String,
    pub uuid: String,
    pub ready: bool,
    pub alive: bool,
    pub away: bool,
    pub spectating_target: Option<u8>,
}

pub struct NetGameDriver<const W: usize, const H: usize> {
    pub engines: SlotMap<PlayerKey, Engine<W, H>>,
    pub local_key: PlayerKey,
    key_by_player_id: HashMap<u8, PlayerKey>,
    on_game_start: Option<Box<dyn FnOnce(u32)>>,
    prev_board_rows: HashMap<PlayerKey, Vec<u64>>,
    pub seq: u32,
    pub last_remote_seq: u32,
    pending_packets: Vec<Vec<u8>>,
    pub room_code: Option<String>,
    pub host_player_id: u8,
    pub player_infos: Vec<PlayerInfo>,
    pub room_settings: RoomSettings,
    pub room_mode: RoomMode,
}

impl<const W: usize, const H: usize> NetGameDriver<W, H> {
    pub fn new(local_engine: Engine<W, H>) -> Self {
        let mut engines = SlotMap::with_key();
        let local_key = engines.insert(local_engine);
        let mut key_by_player_id = HashMap::new();
        key_by_player_id.insert(0, local_key);
        let mut prev_board_rows = HashMap::new();
        prev_board_rows.insert(local_key, vec![0u64; H]);
        NetGameDriver {
            engines,
            local_key,
            key_by_player_id,
            on_game_start: None,
            prev_board_rows,
            seq: 0,
            last_remote_seq: 0,
            pending_packets: Vec::new(),
            room_code: None,
            host_player_id: 0,
            player_infos: Vec::new(),
            room_settings: RoomSettings::default(),
            room_mode: RoomMode::default(),
        }
    }

    pub fn set_on_game_start(&mut self, cb: Box<dyn FnOnce(u32)>) {
        self.on_game_start = Some(cb);
    }

    pub fn add_player(&mut self, engine: Engine<W, H>) -> PlayerKey {
        let key = self.engines.insert(engine);
        let player_id = (0u8..=u8::MAX)
            .find(|id| !self.key_by_player_id.contains_key(id))
            .unwrap_or(self.key_by_player_id.len() as u8);
        self.key_by_player_id.insert(player_id, key);
        self.prev_board_rows.insert(key, vec![0u64; H]);
        key
    }

    pub fn remove_player(&mut self, key: PlayerKey) {
        self.engines.remove(key);
        self.prev_board_rows.remove(&key);
        self.key_by_player_id.retain(|_, &mut v| v != key);
        self.player_infos
            .retain(|p| self.key_by_player_id.contains_key(&p.player_id));
    }

    pub fn player_key_from_id(&self, player_id: u8) -> Option<PlayerKey> {
        self.key_by_player_id.get(&player_id).copied()
    }

    pub fn tick_all(&mut self, delta_ms: u32) {
        // Sequential tick — required for deterministic engine state (D-01).
        for engine in self.engines.values_mut() {
            engine.tick(delta_ms);
        }
    }

    pub fn create_room(&mut self, settings: RoomSettings, host_name: &str, host_uuid: &str) {
        self.room_settings = settings;
        self.room_mode = RoomMode::Lobby;
        self.host_player_id = 0;
        self.player_infos.clear();
        self.player_infos.push(PlayerInfo {
            player_id: 0,
            name: host_name.to_string(),
            uuid: host_uuid.to_string(),
            ready: false,
            alive: true,
            away: false,
            spectating_target: None,
        });
    }

    pub fn join_room(&mut self, player_id: u8, name: &str, uuid: &str) -> Result<(), NetError> {
        if self.player_infos.len() >= self.room_settings.max_players as usize {
            return Err(NetError::Protocol("room full".into()));
        }
        self.player_infos.push(PlayerInfo {
            player_id,
            name: name.to_string(),
            uuid: uuid.to_string(),
            ready: false,
            alive: true,
            away: false,
            spectating_target: None,
        });
        Ok(())
    }

    pub fn set_ready(&mut self, player_id: u8, ready: bool) {
        if let Some(info) = self
            .player_infos
            .iter_mut()
            .find(|p| p.player_id == player_id)
        {
            info.ready = ready;
        }
        if self.room_mode == RoomMode::Lobby && self.all_ready() {
            self.room_mode = RoomMode::Countdown(3);
        }
    }

    pub fn all_ready(&self) -> bool {
        self.player_infos.len() >= 2 && self.player_infos.iter().all(|p| p.ready)
    }

    pub fn tick_countdown(&mut self) -> Option<u8> {
        if let RoomMode::Countdown(remaining) = self.room_mode {
            if remaining == 0 {
                self.start_game();
                return Some(0);
            }
            self.room_mode = RoomMode::Countdown(remaining - 1);
            return Some(remaining - 1);
        }
        None
    }

    pub fn start_game(&mut self) {
        self.room_mode = RoomMode::Playing;
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(42);
        for engine in self.engines.values_mut() {
            engine.reset_with_level(seed, self.room_settings.start_level as u32);
        }
        for info in &mut self.player_infos {
            info.alive = true;
            info.ready = false;
        }
    }

    pub fn handle_player_leave(&mut self, player_id: u8) {
        self.player_infos.retain(|p| p.player_id != player_id);
        if player_id == self.host_player_id {
            self.migrate_host();
        }
    }

    pub fn migrate_host(&mut self) {
        if let Some(next) = self.player_infos.first() {
            self.host_player_id = next.player_id;
        }
    }

    pub fn route_attack(&self, _damage: i32, attacker_id: u8) -> Option<u8> {
        let alive_others: Vec<&PlayerInfo> = self
            .player_infos
            .iter()
            .filter(|p| p.alive && p.player_id != attacker_id)
            .collect();
        if alive_others.is_empty() {
            return None;
        }
        let idx = (attacker_id as usize) % alive_others.len();
        Some(alive_others[idx].player_id)
    }

    pub fn set_away(&mut self, player_id: u8, away: bool) {
        if let Some(info) = self
            .player_infos
            .iter_mut()
            .find(|p| p.player_id == player_id)
        {
            info.away = away;
        }
    }

    pub fn mark_dead(&mut self, player_id: u8) {
        if let Some(info) = self
            .player_infos
            .iter_mut()
            .find(|p| p.player_id == player_id)
        {
            info.alive = false;
        }
        let alive_count = self.player_infos.iter().filter(|p| p.alive).count();
        if alive_count <= 1 {
            self.room_mode = RoomMode::GameOver;
        }
    }

    pub fn reset_to_lobby(&mut self) {
        self.room_mode = RoomMode::Lobby;
        for info in &mut self.player_infos {
            info.ready = false;
            info.alive = true;
        }
    }

    pub fn queue_packet(&mut self, data: Vec<u8>) {
        self.pending_packets.push(data);
    }

    pub fn queue_delta(&mut self, player_key: PlayerKey, local_player_id: u8) {
        if let Some(pkt) = self.delta_encode(player_key, local_player_id)
            && let Ok(data) = bincode::serialize(&pkt)
        {
            self.pending_packets.push(data);
        }
    }

    pub fn flush_batch(&mut self, net: &mut NetworkManager, channel: u8) -> Result<(), NetError> {
        if self.pending_packets.is_empty() {
            return Ok(());
        }
        let batch = PktBatch {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Batch,
                player_id: 0,
            },
            packets: std::mem::take(&mut self.pending_packets),
        };
        net.send_packet(&batch, channel)
    }

    pub fn delta_encode(
        &mut self,
        player_key: PlayerKey,
        local_player_id: u8,
    ) -> Option<PktDeltaSync> {
        let engine = self.engines.get(player_key)?;
        let prev = self.prev_board_rows.get(&player_key)?;

        let changed_rows: Vec<(u8, u64)> = engine
            .state
            .board
            .rows
            .iter()
            .enumerate()
            .filter(|(i, row)| *i < prev.len() && **row != prev[*i])
            .map(|(i, row)| (i as u8, *row))
            .collect();

        let pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: local_player_id,
            },
            seq: self.seq,
            changed_rows,
            piece: engine.state.piece,
            rot: engine.state.rot,
            x: engine.state.x,
            y: engine.state.y,
            hold: engine.state.hold,
            hold_used: engine.state.hold_used,
            next: [
                engine.state.next[0],
                engine.state.next[1],
                engine.state.next[2],
            ],
        };

        if let Some(cache) = self.prev_board_rows.get_mut(&player_key) {
            cache.copy_from_slice(&engine.state.board.rows);
        }
        self.seq += 1;

        Some(pkt)
    }

    pub fn send_delta_sync(
        &mut self,
        net: &mut NetworkManager,
        player_key: PlayerKey,
    ) -> Result<bool, NetError> {
        if let Some(pkt) = self.delta_encode(player_key, net.local_player_id) {
            net.send_packet(&pkt, 2)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn send_resync_request(
        &self,
        net: &mut NetworkManager,
        last_good_seq: u32,
    ) -> Result<(), NetError> {
        let pkt = PktResyncRequest {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ResyncRequest,
                player_id: net.local_player_id,
            },
            last_good_seq,
        };
        net.send_packet(&pkt, 0)
    }

    pub fn handle_packet(&mut self, data: &[u8]) -> Result<(), NetError> {
        let header: PacketHeader = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;

        if header.version != PROTOCOL_VERSION {
            return Err(NetError::Protocol(format!(
                "protocol version mismatch: got {}, expected {}",
                header.version, PROTOCOL_VERSION
            )));
        }

        match header.packet_type {
            PacketType::Batch => {
                let batch: PktBatch = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                for inner in &batch.packets {
                    self.handle_packet(inner)?;
                }
            }
            PacketType::GameStart => {
                let pkt: PktGameStart = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(cb) = self.on_game_start.take() {
                    cb(pkt.random_seed);
                }
            }
            PacketType::PlayerAction => {
                let pkt: PktPlayerAction =
                    deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(key) = self.player_key_from_id(header.player_id)
                    && let Some(engine) = self.engines.get_mut(key)
                {
                    engine.handle_action(pkt.action);
                }
            }
            PacketType::PlayerAttack => {
                let pkt: PktPlayerAttack =
                    deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(key) = self.player_key_from_id(header.player_id)
                    && let Some(engine) = self.engines.get_mut(key)
                {
                    engine.state.pending_garbage =
                        engine.state.pending_garbage.saturating_add(pkt.lines);
                }
            }
            PacketType::StateSync => {
                let pkt: PktStateSync = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(key) = self.player_key_from_id(header.player_id) {
                    if let Some(engine) = self.engines.get_mut(key) {
                        for (i, &val) in pkt.board_rows.iter().enumerate() {
                            if i < H {
                                engine.state.board.rows[i] = val;
                            }
                        }
                        engine.state.piece = pkt.piece;
                        engine.state.rot = pkt.rot;
                        engine.state.x = pkt.x;
                        engine.state.y = pkt.y;
                        engine.state.hold = pkt.hold;
                        engine.state.hold_used = pkt.hold_used;
                        engine.state.next[0] = pkt.next[0];
                        engine.state.next[1] = pkt.next[1];
                        engine.state.next[2] = pkt.next[2];
                        // next[3],[4] preserved: legacy PktStateSync carries only 3 pieces
                        engine.state.pending_garbage = pkt.pending_garbage;
                        engine.state.rng = pkt.rng_state;
                        engine.game_over = false;
                    }
                    if let Some(cache) = self.prev_board_rows.get_mut(&key)
                        && let Some(engine) = self.engines.get(key)
                    {
                        cache.copy_from_slice(&engine.state.board.rows);
                    }
                }
            }
            PacketType::DeltaSync => {
                let pkt: PktDeltaSync = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                let expected_seq = self.last_remote_seq + 1;
                if self.last_remote_seq == 0 && pkt.seq == 0 {
                } else if pkt.seq != expected_seq {
                    return Err(NetError::Protocol(format!(
                        "seq gap: expected {}, got {} (resync needed)",
                        expected_seq, pkt.seq
                    )));
                }
                if let Some(key) = self.player_key_from_id(header.player_id)
                    && let Some(engine) = self.engines.get_mut(key)
                {
                    for &(row_idx, new_val) in &pkt.changed_rows {
                        if (row_idx as usize) < H {
                            engine.state.board.rows[row_idx as usize] = new_val;
                        }
                    }
                    engine.state.piece = pkt.piece;
                    engine.state.rot = pkt.rot;
                    engine.state.x = pkt.x;
                    engine.state.y = pkt.y;
                    engine.state.hold = pkt.hold;
                    engine.state.hold_used = pkt.hold_used;
                    engine.state.next[0] = pkt.next[0];
                    engine.state.next[1] = pkt.next[1];
                    engine.state.next[2] = pkt.next[2];
                    // next[3],[4] preserved: legacy PktDeltaSync carries only 3 pieces
                    self.last_remote_seq = pkt.seq;
                }
            }
            PacketType::ResyncRequest => {}
            PacketType::PlayerStateSync => {
                let pkt: PktPlayerStateSync =
                    deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(info) = self
                    .player_infos
                    .iter_mut()
                    .find(|p| p.player_id == pkt.target_player_id)
                {
                    info.alive = pkt.alive;
                    info.spectating_target = pkt.spectating_target;
                }
            }
            PacketType::GameOver => {
                let pkt: PktGameOver = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                self.room_mode = RoomMode::GameOver;
                self.player_infos.iter_mut().for_each(|p| {
                    if p.player_id != pkt.winner_player_id {
                        p.alive = false;
                    }
                });
            }
            PacketType::Reconnect => {
                let pkt: PktReconnect = deser(data).map_err(|e| NetError::Decode(e.to_string()))?;
                if let Some(key) = self.player_key_from_id(header.player_id) {
                    self.prev_board_rows.remove(&key);
                    self.prev_board_rows.insert(key, vec![0u64; H]);
                }
                let _ = pkt;
            }
            PacketType::PlayerAway => {
                if let Some(info) = self
                    .player_infos
                    .iter_mut()
                    .find(|p| p.player_id == header.player_id)
                {
                    info.away = true;
                }
            }
            PacketType::HostMigrate | PacketType::ReconnectAck | PacketType::Resume => {}
            _ => {}
        }
        Ok(())
    }

    pub fn process_network(&mut self, net: &mut NetworkManager) {
        // renet channels 0-2 correspond to reliable/unreliable/broadcast;
        // channel 3 is unused and iterated only for future extension.
        for channel in 0..3 {
            for data in net.receive_messages(channel) {
                if let Err(e) = self.handle_packet(&data) {
                    eprintln!("packet error: {e}");
                }
            }
        }
    }

    pub fn send_action(
        &mut self,
        net: &mut NetworkManager,
        action: Action,
    ) -> Result<(), NetError> {
        let pkt = PktPlayerAction {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAction,
                player_id: net.local_player_id,
            },
            action,
        };
        net.send_packet(&pkt, 1)
    }

    pub fn send_attack(
        &mut self,
        net: &mut NetworkManager,
        lines: u8,
        hole_x: u8,
    ) -> Result<(), NetError> {
        let pkt = PktPlayerAttack {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAttack,
                player_id: net.local_player_id,
            },
            lines,
            hole_x,
        };
        net.send_packet(&pkt, 1)
    }

    pub fn send_state_sync(
        &mut self,
        net: &mut NetworkManager,
        player_key: PlayerKey,
    ) -> Result<(), NetError> {
        let engine = self
            .engines
            .get(player_key)
            .ok_or(NetError::Protocol("invalid player key".into()))?;
        let pkt = PktStateSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSync,
                player_id: net.local_player_id,
            },
            board_rows: engine.state.board.rows.to_vec(),
            piece: engine.state.piece,
            rot: engine.state.rot,
            x: engine.state.x,
            y: engine.state.y,
            hold: engine.state.hold,
            hold_used: engine.state.hold_used,
            next: [
                engine.state.next[0],
                engine.state.next[1],
                engine.state.next[2],
            ],
            pending_garbage: engine.state.pending_garbage,
            rng_state: engine.state.rng,
        };
        net.send_packet(&pkt, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_action_header() {
        let pkt = PktPlayerAction {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAction,
                player_id: 0,
            },
            action: Action::MoveLeft,
        };
        let bytes = bincode::serialize(&pkt).unwrap();
        let decoded: PktPlayerAction = bincode::deserialize(&bytes).unwrap();
        assert_eq!(decoded.header.player_id, 0);
        assert_eq!(decoded.action, Action::MoveLeft);
    }

    #[test]
    fn test_slotmap_multi_player_routing() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        let remote_engine = Engine::<10, 20>::new();
        let remote_key = driver.add_player(remote_engine);

        assert_eq!(driver.player_key_from_id(0), Some(driver.local_key));
        assert_eq!(driver.player_key_from_id(1), Some(remote_key));
        assert_eq!(driver.player_key_from_id(2), None);

        let pkt = PktPlayerAction {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAction,
                player_id: 1,
            },
            action: Action::MoveLeft,
        };
        let data = bincode::serialize(&pkt).unwrap();
        driver.handle_packet(&data).unwrap();
    }

    #[test]
    fn test_player_attack_routes_to_local() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        let remote_engine = Engine::<10, 20>::new();
        driver.add_player(remote_engine);

        let initial_garbage = driver.engines[driver.local_key].state.pending_garbage;
        // Attack with player_id=0 targets local engine
        let pkt = PktPlayerAttack {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAttack,
                player_id: 0,
            },
            lines: 3,
            hole_x: 5,
        };
        let data = bincode::serialize(&pkt).unwrap();
        driver.handle_packet(&data).unwrap();
        assert_eq!(
            driver.engines[driver.local_key].state.pending_garbage,
            initial_garbage + 3
        );
    }

    #[test]
    fn test_remove_player() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        let remote_engine = Engine::<10, 20>::new();
        let remote_key = driver.add_player(remote_engine);

        assert!(driver.engines.contains_key(remote_key));
        driver.remove_player(remote_key);
        assert!(!driver.engines.contains_key(remote_key));
        assert!(driver.engines.contains_key(driver.local_key));
    }

    #[test]
    fn test_delta_encode_no_change() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        let first = driver.delta_encode(driver.local_key, 0);
        assert!(first.is_some());
        assert_eq!(first.unwrap().seq, 0);

        let second = driver.delta_encode(driver.local_key, 0);
        assert!(second.is_some());
        assert!(second.as_ref().unwrap().changed_rows.is_empty());
    }

    #[test]
    fn test_delta_encode_single_row() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        let _ = driver.delta_encode(driver.local_key, 0);

        driver.engines[driver.local_key].state.board.rows[19] = 0x3FF;

        let delta = driver.delta_encode(driver.local_key, 0).unwrap();
        assert_eq!(delta.changed_rows.len(), 1);
        assert_eq!(delta.changed_rows[0], (19, 0x3FF));
    }

    #[test]
    fn test_seq_increment() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);

        assert_eq!(driver.seq, 0);
        driver.delta_encode(driver.local_key, 0);
        assert_eq!(driver.seq, 1);
        driver.delta_encode(driver.local_key, 0);
        assert_eq!(driver.seq, 2);
    }

    #[test]
    fn test_handle_delta_sync_apply() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);
        let remote_engine = Engine::<10, 20>::new();
        driver.add_player(remote_engine);

        let pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: 1,
            },
            seq: 0,
            changed_rows: vec![(19, 0x3FF)],
            piece: tetris_core::types::Piece::T,
            rot: tetris_core::types::Rot::R0,
            x: 3,
            y: 0,
            hold: tetris_core::types::Piece::I,
            hold_used: false,
            next: [tetris_core::types::Piece::I; 3],
        };
        let data = bincode::serialize(&pkt).unwrap();
        driver.handle_packet(&data).unwrap();

        let remote_key = driver.player_key_from_id(1).unwrap();
        assert_eq!(driver.engines[remote_key].state.board.rows[19], 0x3FF);
    }

    #[test]
    fn test_tick_all_deterministic() {
        let mut driver = NetGameDriver::<10, 20>::new(Engine::new());
        for i in 0..7 {
            let mut engine = Engine::<10, 20>::new();
            engine.reset((42 + i * 100) as u32);
            driver.add_player(engine);
        }
        assert_eq!(driver.engines.len(), 8);

        let mut driver2 = NetGameDriver::<10, 20>::new(Engine::new());
        for i in 0..7 {
            let mut engine = Engine::<10, 20>::new();
            engine.reset((42 + i * 100) as u32);
            driver2.add_player(engine);
        }

        driver.tick_all(16);
        driver.tick_all(16);
        driver.tick_all(16);
        driver2.tick_all(16);
        driver2.tick_all(16);
        driver2.tick_all(16);

        let all_keys: Vec<_> = driver.engines.keys().collect();
        for key in all_keys {
            assert_eq!(
                driver.engines[key].state_hash(),
                driver2.engines[key].state_hash(),
                "engine state hash mismatch for player {:?}",
                key
            );
        }
    }

    #[test]
    fn test_queue_and_flush() {
        let mut driver = NetGameDriver::<10, 20>::new(Engine::new());
        driver.queue_packet(vec![1, 2, 3]);
        driver.queue_packet(vec![4, 5, 6]);
        assert_eq!(driver.pending_packets.len(), 2);

        let batch = PktBatch {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Batch,
                player_id: 0,
            },
            packets: std::mem::take(&mut driver.pending_packets),
        };
        assert_eq!(batch.packets.len(), 2);
        assert!(driver.pending_packets.is_empty());
    }

    #[test]
    fn test_handle_delta_sync_seq_gap() {
        let local_engine = Engine::<10, 20>::new();
        let mut driver = NetGameDriver::new(local_engine);
        let remote_engine = Engine::<10, 20>::new();
        driver.add_player(remote_engine);

        let pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: 1,
            },
            seq: 0,
            changed_rows: vec![],
            piece: tetris_core::types::Piece::T,
            rot: tetris_core::types::Rot::R0,
            x: 3,
            y: 0,
            hold: tetris_core::types::Piece::I,
            hold_used: false,
            next: [tetris_core::types::Piece::I; 3],
        };
        let data = bincode::serialize(&pkt).unwrap();
        driver.handle_packet(&data).unwrap();

        let gap_pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: 1,
            },
            seq: 5,
            changed_rows: vec![],
            piece: tetris_core::types::Piece::T,
            rot: tetris_core::types::Rot::R0,
            x: 3,
            y: 0,
            hold: tetris_core::types::Piece::I,
            hold_used: false,
            next: [tetris_core::types::Piece::I; 3],
        };
        let data2 = bincode::serialize(&gap_pkt).unwrap();
        let result = driver.handle_packet(&data2);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("seq gap"));
    }
}

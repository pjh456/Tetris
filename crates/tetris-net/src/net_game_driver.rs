use std::collections::HashMap;

use slotmap::{new_key_type, SlotMap};
use tetris_core::engine::{Action, Engine};

use crate::network_manager::NetworkManager;
use crate::protocol::*;

new_key_type! { pub struct PlayerKey; }

pub struct NetGameDriver<const W: usize, const H: usize> {
    pub engines: SlotMap<PlayerKey, Engine<W, H>>,
    pub local_key: PlayerKey,
    key_by_player_id: Vec<PlayerKey>,
    on_game_start: Option<Box<dyn FnOnce(u32)>>,
    prev_board_rows: HashMap<PlayerKey, Vec<u64>>,
    pub seq: u32,
    pub last_remote_seq: u32,
}

impl<const W: usize, const H: usize> NetGameDriver<W, H> {
    pub fn new(local_engine: Engine<W, H>) -> Self {
        let mut engines = SlotMap::with_key();
        let local_key = engines.insert(local_engine);
        let key_by_player_id = vec![local_key];
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
        }
    }

    pub fn set_on_game_start(&mut self, cb: Box<dyn FnOnce(u32)>) {
        self.on_game_start = Some(cb);
    }

    pub fn add_player(&mut self, engine: Engine<W, H>) -> PlayerKey {
        let key = self.engines.insert(engine);
        self.key_by_player_id.push(key);
        self.prev_board_rows.insert(key, vec![0u64; H]);
        key
    }

    pub fn remove_player(&mut self, key: PlayerKey) {
        self.engines.remove(key);
        self.prev_board_rows.remove(&key);
    }

    pub fn player_key_from_id(&self, player_id: u8) -> Option<PlayerKey> {
        self.key_by_player_id.get(player_id as usize).copied()
    }

    pub fn delta_encode(&mut self, player_key: PlayerKey, local_player_id: u8) -> Option<PktDeltaSync> {
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
    ) -> Result<bool, String> {
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
    ) -> Result<(), String> {
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

    pub fn handle_packet(&mut self, data: &[u8]) -> Result<(), String> {
        let header: PacketHeader =
            bincode::deserialize(data).map_err(|e| format!("decode header error: {}", e))?;

        match header.packet_type {
            PacketType::GameStart => {
                let pkt: PktGameStart = bincode::deserialize(data)
                    .map_err(|e| format!("decode GameStart error: {}", e))?;
                if let Some(cb) = self.on_game_start.take() {
                    cb(pkt.random_seed);
                }
            }
            PacketType::PlayerAction => {
                let pkt: PktPlayerAction = bincode::deserialize(data)
                    .map_err(|e| format!("decode PlayerAction error: {}", e))?;
                if let Some(key) = self.player_key_from_id(header.player_id) {
                    if let Some(engine) = self.engines.get_mut(key) {
                        engine.handle_action(pkt.action);
                    }
                }
            }
            PacketType::PlayerAttack => {
                let pkt: PktPlayerAttack = bincode::deserialize(data)
                    .map_err(|e| format!("decode PlayerAttack error: {}", e))?;
                if let Some(engine) = self.engines.get_mut(self.local_key) {
                    engine.state.pending_garbage += pkt.lines;
                }
            }
            PacketType::StateSync => {
                let pkt: PktStateSync = bincode::deserialize(data)
                    .map_err(|e| format!("decode StateSync error: {}", e))?;
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
                        engine.state.next = [
                            pkt.next[0],
                            pkt.next[1],
                            pkt.next[2],
                            pkt.next[0],
                            pkt.next[0],
                        ];
                        engine.state.pending_garbage = pkt.pending_garbage;
                        engine.state.rng = pkt.rng_state;
                    }
                    if let Some(cache) = self.prev_board_rows.get_mut(&key) {
                        if let Some(engine) = self.engines.get(key) {
                            cache.copy_from_slice(&engine.state.board.rows);
                        }
                    }
                }
            }
            PacketType::DeltaSync => {
                let pkt: PktDeltaSync = bincode::deserialize(data)
                    .map_err(|e| format!("decode DeltaSync error: {}", e))?;
                let expected_seq = self.last_remote_seq + 1;
                if self.last_remote_seq == 0 && pkt.seq == 0 {
                } else if pkt.seq != expected_seq {
                    return Err(format!(
                        "seq gap: expected {}, got {} (resync needed)",
                        expected_seq, pkt.seq
                    ));
                }
                if let Some(key) = self.player_key_from_id(header.player_id) {
                    if let Some(engine) = self.engines.get_mut(key) {
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
                        engine.state.next = [
                            pkt.next[0],
                            pkt.next[1],
                            pkt.next[2],
                            pkt.next[0],
                            pkt.next[0],
                        ];
                    }
                }
                self.last_remote_seq = pkt.seq;
            }
            PacketType::ResyncRequest => {}
            _ => {}
        }
        Ok(())
    }

    pub fn process_network(&mut self, net: &mut NetworkManager) {
        for channel in 0..3 {
            for data in net.receive_messages(channel) {
                let _ = self.handle_packet(&data);
            }
        }
    }

    pub fn send_action(&mut self, net: &mut NetworkManager, action: Action) -> Result<(), String> {
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
    ) -> Result<(), String> {
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
    ) -> Result<(), String> {
        let engine = self
            .engines
            .get(player_key)
            .ok_or_else(|| "invalid player key".to_string())?;
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
        let pkt = PktPlayerAttack {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAttack,
                player_id: 1,
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
        assert!(result.unwrap_err().contains("seq gap"));
    }
}

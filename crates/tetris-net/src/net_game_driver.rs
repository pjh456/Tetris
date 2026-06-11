use serde::Serialize;
use tetris_core::engine::{Action, Engine};

use crate::network_manager::NetworkManager;
use crate::protocol::*;

pub struct NetGameDriver<const W: usize, const H: usize> {
    pub local: Engine<W, H>,
    pub remote: Engine<W, H>,
    on_game_start: Option<Box<dyn FnOnce(u32)>>,
}

impl<const W: usize, const H: usize> NetGameDriver<W, H> {
    pub fn new(local: Engine<W, H>, remote: Engine<W, H>) -> Self {
        NetGameDriver {
            local,
            remote,
            on_game_start: None,
        }
    }

    pub fn set_on_game_start(&mut self, cb: Box<dyn FnOnce(u32)>) {
        self.on_game_start = Some(cb);
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
                self.remote.handle_action(pkt.action);
            }
            PacketType::PlayerAttack => {
                let pkt: PktPlayerAttack = bincode::deserialize(data)
                    .map_err(|e| format!("decode PlayerAttack error: {}", e))?;
                self.local.state.pending_garbage += pkt.lines;
            }
            PacketType::StateSync => {
                let pkt: PktStateSync = bincode::deserialize(data)
                    .map_err(|e| format!("decode StateSync error: {}", e))?;
                for (i, &val) in pkt.board_rows.iter().enumerate() {
                    if i < H {
                        self.remote.state.board.rows[i] = val;
                    }
                }
                self.remote.state.piece = pkt.piece;
                self.remote.state.rot = pkt.rot;
                self.remote.state.x = pkt.x;
                self.remote.state.y = pkt.y;
                self.remote.state.hold = pkt.hold;
                self.remote.state.hold_used = pkt.hold_used;
                self.remote.state.next = [
                    pkt.next[0],
                    pkt.next[1],
                    pkt.next[2],
                    pkt.next[0],
                    pkt.next[0],
                ];
                self.remote.state.pending_garbage = pkt.pending_garbage;
                self.remote.state.rng = pkt.rng_state;
            }
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

    pub fn send_state_sync(&mut self, net: &mut NetworkManager) -> Result<(), String> {
        let pkt = PktStateSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSync,
                player_id: net.local_player_id,
            },
            board_rows: self.local.state.board.rows.to_vec(),
            piece: self.local.state.piece,
            rot: self.local.state.rot,
            x: self.local.state.x,
            y: self.local.state.y,
            hold: self.local.state.hold,
            hold_used: self.local.state.hold_used,
            next: [
                self.local.state.next[0],
                self.local.state.next[1],
                self.local.state.next[2],
            ],
            pending_garbage: self.local.state.pending_garbage,
            rng_state: self.local.state.rng,
        };
        net.send_packet(&pkt, 2)
    }

    pub fn send<T: Serialize>(&self, _pkt: &T, _channel: u8) -> Result<(), String> {
        Ok(())
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
}

use serde::{Deserialize, Serialize};
use tetris_core::engine::Action;
use tetris_core::types::{Piece, Rot};

pub const PROTOCOL_VERSION: u8 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    ClientJoin = 1,
    ServerAccept,
    HostStart,
    GameStart,
    PlayerAction,
    PlayerAttack,
    StateSync,
    GameOver,
    VersionError,
    DeltaSync = 10,
    ResyncRequest = 11,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketHeader {
    pub version: u8,
    pub packet_type: PacketType,
    pub player_id: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktClientJoin {
    pub header: PacketHeader,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktServerAccept {
    pub header: PacketHeader,
    pub assigned_player_id: u8,
    pub max_players: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktVersionError {
    pub header: PacketHeader,
    pub server_version: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktGameStart {
    pub header: PacketHeader,
    pub random_seed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerAction {
    pub header: PacketHeader,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerAttack {
    pub header: PacketHeader,
    pub lines: u8,
    pub hole_x: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktGameOver {
    pub header: PacketHeader,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktStateSync {
    pub header: PacketHeader,
    pub board_rows: Vec<u64>,
    pub piece: Piece,
    pub rot: Rot,
    pub x: i8,
    pub y: i8,
    pub hold: Piece,
    pub hold_used: bool,
    pub next: [Piece; 3],
    pub pending_garbage: u8,
    pub rng_state: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktBatch {
    pub packets: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktDeltaSync {
    pub header: PacketHeader,
    pub seq: u32,
    pub changed_rows: Vec<(u8, u64)>,
    pub piece: Piece,
    pub rot: Rot,
    pub x: i8,
    pub y: i8,
    pub hold: Piece,
    pub hold_used: bool,
    pub next: [Piece; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktResyncRequest {
    pub header: PacketHeader,
    pub last_good_seq: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::engine::Action;

    fn bincode_round_trip<T: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + PartialEq>(
        value: &T,
    ) {
        let bytes = bincode::serialize(value).unwrap();
        let decoded: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(&decoded, value);
    }

    #[test]
    fn test_packet_header_round_trip() {
        let hdr = PacketHeader {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::ClientJoin,
            player_id: 0,
        };
        bincode_round_trip(&hdr);
    }

    #[test]
    fn test_player_action_round_trip() {
        let pkt = PktPlayerAction {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAction,
                player_id: 1,
            },
            action: Action::HardDrop,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_player_attack_round_trip() {
        let pkt = PktPlayerAttack {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAttack,
                player_id: 0,
            },
            lines: 2,
            hole_x: 5,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_game_start_round_trip() {
        let pkt = PktGameStart {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::GameStart,
                player_id: 0,
            },
            random_seed: 42,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_player_state_sync_round_trip() {
        let pkt = PktStateSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSync,
                player_id: 0,
            },
            board_rows: vec![0; 20],
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 3],
            pending_garbage: 0,
            rng_state: 12345,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_server_accept_round_trip() {
        let pkt = PktServerAccept {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ServerAccept,
                player_id: 0,
            },
            assigned_player_id: 1,
            max_players: 2,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_protocol_version() {
        assert_eq!(PROTOCOL_VERSION, 0x10);
    }

    #[test]
    fn test_client_join_type() {
        assert_eq!(PacketType::ClientJoin as u8, 1);
    }

    #[test]
    fn test_delta_sync_round_trip() {
        let pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: 0,
            },
            seq: 42,
            changed_rows: vec![(5, 0xFF), (19, 0x3FF)],
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::S, Piece::Z, Piece::L],
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_delta_sync_empty_rows() {
        let pkt = PktDeltaSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::DeltaSync,
                player_id: 0,
            },
            seq: 0,
            changed_rows: vec![],
            piece: Piece::O,
            rot: Rot::R0,
            x: 4,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 3],
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_resync_request_round_trip() {
        let pkt = PktResyncRequest {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ResyncRequest,
                player_id: 1,
            },
            last_good_seq: 100,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_batch_round_trip() {
        let inner1 = bincode::serialize(&PktPlayerAction {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAction,
                player_id: 0,
            },
            action: Action::MoveLeft,
        })
        .unwrap();
        let inner2 = bincode::serialize(&PktPlayerAttack {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerAttack,
                player_id: 0,
            },
            lines: 2,
            hole_x: 3,
        })
        .unwrap();
        let batch = PktBatch {
            packets: vec![inner1, inner2],
        };
        bincode_round_trip(&batch);
        assert_eq!(batch.packets.len(), 2);
    }

    #[test]
    fn test_delta_sync_packet_type() {
        assert_eq!(PacketType::DeltaSync as u8, 10);
        assert_eq!(PacketType::ResyncRequest as u8, 11);
    }
}

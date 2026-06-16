use serde::{Deserialize, Serialize};
use tetris_core::engine::Action;
use tetris_core::types::{Piece, Rot};

use crate::newtypes::{KeyAction, PlayerSlot, Seed, TickNumber};

pub const PROTOCOL_VERSION: u8 = 0x11;

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
    CreateRoom = 12,
    JoinRoom = 13,
    PlayerReady = 14,
    PlayerLeave = 15,
    RoomSettings = 16,
    ChatMessage = 17,
    StartCountdown = 18,
    HostMigrate = 19,
    PlayerAway = 20,
    SpectateSwitch = 21,
    RoomSnapshot = 22,
    Replay = 23,
    ServerReplay = 24,
    StateHash = 25,
    StateSnapshot = 26,
    Reconnect = 27,
    ReconnectAck = 28,
    Resume = 29,
    Ige = 30,
    IncomingGarbage = 31,
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
pub struct PktIncomingGarbage {
    pub header: PacketHeader,
    pub incoming_lines: u8,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktCreateRoom {
    pub header: PacketHeader,
    pub max_players: u8,
    pub start_level: u8,
    pub attack_mult: f32,
    pub garbage_delay_secs: u8,
    pub allow_hold: bool,
    pub host_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktJoinRoom {
    pub header: PacketHeader,
    pub room_code: String,
    pub player_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktRoomSettings {
    pub header: PacketHeader,
    pub max_players: u8,
    pub start_level: u8,
    pub attack_mult: f32,
    pub garbage_delay_secs: u8,
    pub allow_hold: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerReady {
    pub header: PacketHeader,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerLeave {
    pub header: PacketHeader,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktChatMessage {
    pub header: PacketHeader,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktStartCountdown {
    pub header: PacketHeader,
    pub remaining_secs: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktHostMigrate {
    pub header: PacketHeader,
    pub new_host_player_id: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerAway {
    pub header: PacketHeader,
    pub away: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktSpectateSwitch {
    pub header: PacketHeader,
    pub target_player_id: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomPlayerSnapshot {
    pub player_id: u8,
    pub name: String,
    pub ready: bool,
    pub alive: bool,
    pub away: bool,
    pub is_host: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktRoomSnapshot {
    pub header: PacketHeader,
    pub room_code: String,
    pub players: Vec<RoomPlayerSnapshot>,
}

// ── 0x11 protocol types (per D-02, D-05, D-06, D-10, D-12) ──

/// Client-to-server raw key event. Per D-02.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputEvent {
    pub key: KeyAction,
    pub pressed: bool,
    pub tick: TickNumber,
    pub subframe: f32,
}

/// Client-to-server batched input events. Per D-05.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktReplay {
    pub header: PacketHeader,
    pub events: Vec<InputEvent>,
    pub start_tick: TickNumber,
}

/// Server-to-client opponent replay events with optional ige garbage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktServerReplay {
    pub header: PacketHeader,
    pub source_player: PlayerSlot,
    pub events: Vec<InputEvent>,
    pub ige_garbage_lines: u8,
    pub ige_hole_x: u8,
}

/// Server broadcast of state hash for fast divergence detection. Per D-06.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktStateHash {
    pub header: PacketHeader,
    pub tick: TickNumber,
    pub hash: u32,
}

/// Full state snapshot for catch-up or deep resync. Per D-12.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktStateSnapshot {
    pub header: PacketHeader,
    pub tick: TickNumber,
    pub board_rows: Vec<u64>,
    pub piece: Piece,
    pub rot: Rot,
    pub x: i8,
    pub y: i8,
    pub hold: Piece,
    pub hold_used: bool,
    pub next: [Piece; 5],
    pub rng_state: u32,
    pub combo: i32,
    pub b2b: bool,
    pub pending_garbage: u8,
    pub seed: Seed,
}

/// Client-to-server reconnect request with hash ladder. Per D-10, D-11.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktReconnect {
    pub header: PacketHeader,
    pub last_good_tick: TickNumber,
    pub client_hashes: Vec<(TickNumber, u32)>,
}

/// Server response to reconnect: divergence point + catch-up events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktReconnectAck {
    pub header: PacketHeader,
    pub divergence_tick: TickNumber,
    pub replay_events: Vec<PktServerReplay>,
}

/// Resume connection with stored session state. Per D-10.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktResume {
    pub header: PacketHeader,
    pub socket_id: String,
    pub resume_token: String,
}

// ── Deprecated (retained for backward-compat reference) ──
// Note: PktPlayerAction is superseded by PktReplay with InputEvent (0x11).
// PktStateSync is superseded by PktStateSnapshot (full) / PktServerReplay (incremental).

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
        assert_eq!(PROTOCOL_VERSION, 0x11);
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

    #[test]
    fn test_create_room_round_trip() {
        let pkt = PktCreateRoom {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::CreateRoom,
                player_id: 0,
            },
            max_players: 4,
            start_level: 1,
            attack_mult: 1.0,
            garbage_delay_secs: 1,
            allow_hold: true,
            host_name: "Alice".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_join_room_round_trip() {
        let pkt = PktJoinRoom {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::JoinRoom,
                player_id: 0,
            },
            room_code: "ABCD".into(),
            player_name: "Bob".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_player_ready_round_trip() {
        let pkt = PktPlayerReady {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerReady,
                player_id: 1,
            },
            ready: true,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_chat_message_round_trip() {
        let pkt = PktChatMessage {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ChatMessage,
                player_id: 1,
            },
            message: "hello".into(),
            timestamp: "2026-06-13T00:00:00Z".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_start_countdown_round_trip() {
        let pkt = PktStartCountdown {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StartCountdown,
                player_id: 0,
            },
            remaining_secs: 3,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_host_migrate_round_trip() {
        let pkt = PktHostMigrate {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::HostMigrate,
                player_id: 0,
            },
            new_host_player_id: 2,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_spectate_switch_round_trip() {
        let pkt = PktSpectateSwitch {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::SpectateSwitch,
                player_id: 1,
            },
            target_player_id: 3,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_room_packet_types() {
        assert_eq!(PacketType::CreateRoom as u8, 12);
        assert_eq!(PacketType::SpectateSwitch as u8, 21);
        assert_eq!(PacketType::RoomSnapshot as u8, 22);
    }

    #[test]
    fn test_room_snapshot_round_trip() {
        let pkt = PktRoomSnapshot {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::RoomSnapshot,
                player_id: 0,
            },
            room_code: "ABCD".into(),
            players: vec![RoomPlayerSnapshot {
                player_id: 0,
                name: "Alice".into(),
                ready: true,
                alive: true,
                away: false,
                is_host: true,
            }],
        };
        bincode_round_trip(&pkt);
    }

    // ── 0x11 protocol type tests ──

    #[test]
    fn test_input_event_round_trip() {
        let ev = InputEvent {
            key: KeyAction::KeyLeft,
            pressed: true,
            tick: TickNumber(5),
            subframe: 0.5,
        };
        bincode_round_trip(&ev);
    }

    #[test]
    fn test_input_event_key_release_round_trip() {
        let ev = InputEvent {
            key: KeyAction::KeyHardDrop,
            pressed: false,
            tick: TickNumber(0),
            subframe: 0.0,
        };
        bincode_round_trip(&ev);
    }

    #[test]
    fn test_pkt_replay_round_trip() {
        let pkt = PktReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Replay,
                player_id: 0,
            },
            events: vec![
                InputEvent {
                    key: KeyAction::KeyLeft,
                    pressed: true,
                    tick: TickNumber(1),
                    subframe: 0.1,
                },
                InputEvent {
                    key: KeyAction::KeyHardDrop,
                    pressed: true,
                    tick: TickNumber(2),
                    subframe: 0.5,
                },
            ],
            start_tick: TickNumber(1),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_server_replay_round_trip() {
        let pkt = PktServerReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ServerReplay,
                player_id: 0,
            },
            source_player: PlayerSlot(1),
            events: vec![InputEvent {
                key: KeyAction::KeyRotateCW,
                pressed: true,
                tick: TickNumber(3),
                subframe: 0.2,
            }],
            ige_garbage_lines: 2,
            ige_hole_x: 5,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_state_hash_round_trip() {
        let pkt = PktStateHash {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateHash,
                player_id: 0,
            },
            tick: TickNumber(100),
            hash: 0xDEAD_BEEF,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_state_snapshot_round_trip() {
        let pkt = PktStateSnapshot {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSnapshot,
                player_id: 0,
            },
            tick: TickNumber(50),
            board_rows: vec![0; 20],
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::T, Piece::S, Piece::Z, Piece::L, Piece::J],
            rng_state: 12345,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            seed: Seed(42),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_reconnect_round_trip() {
        let pkt = PktReconnect {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Reconnect,
                player_id: 1,
            },
            last_good_tick: TickNumber(99),
            client_hashes: vec![(TickNumber(0), 0xAAAA), (TickNumber(100), 0xBBBB)],
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_reconnect_ack_round_trip() {
        let pkt = PktReconnectAck {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ReconnectAck,
                player_id: 0,
            },
            divergence_tick: TickNumber(99),
            replay_events: vec![],
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_resume_round_trip() {
        let pkt = PktResume {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Resume,
                player_id: 1,
            },
            socket_id: "abc".into(),
            resume_token: "xyz".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_protocol_version_0x11() {
        assert_eq!(PROTOCOL_VERSION, 0x11);
    }

    #[test]
    fn test_new_packet_types() {
        assert_eq!(PacketType::Replay as u8, 23);
        assert_eq!(PacketType::ServerReplay as u8, 24);
        assert_eq!(PacketType::StateHash as u8, 25);
        assert_eq!(PacketType::StateSnapshot as u8, 26);
        assert_eq!(PacketType::Reconnect as u8, 27);
        assert_eq!(PacketType::ReconnectAck as u8, 28);
        assert_eq!(PacketType::Resume as u8, 29);
        assert_eq!(PacketType::Ige as u8, 30);
    }
}

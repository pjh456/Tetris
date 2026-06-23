use serde::{Deserialize, Serialize};
use tetris_core::types::{Piece, Rot};

use crate::newtypes::{KeyAction, PlayerSlot, Seed, TickNumber};

pub const PROTOCOL_VERSION: u8 = 0x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    ClientJoin = 1,
    ServerAccept = 2,
    GameStart = 4,
    GameOver = 8,
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
    IncomingGarbage = 31,
    PlayerStatus = 32,
    Batch = 33,
    AddBot = 34,
    CountdownCancel = 35,
    KickPlayer = 36,
    RemoveBot = 37,
    Standings = 38,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketHeader {
    pub version: u8,
    pub packet_type: PacketType,
    pub player_id: u8,
}

impl PacketHeader {
    pub fn new(packet_type: PacketType, player_id: u8) -> Self {
        PacketHeader {
            version: PROTOCOL_VERSION,
            packet_type,
            player_id,
        }
    }
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
    /// Server-issued unpredictable resume token. Delivered only to the owning
    /// client; used to authenticate reconnect/resume requests (anti-spoofing).
    pub resume_token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktGameStart {
    pub header: PacketHeader,
    pub random_seed: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktIncomingGarbage {
    pub header: PacketHeader,
    pub incoming_lines: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktGameOver {
    pub header: PacketHeader,
    pub winner_player_id: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktPlayerStatus {
    pub header: PacketHeader,
    pub target_player_id: u8,
    pub alive: bool,
    pub spectating: bool,
    pub spectating_target: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktBatch {
    pub header: PacketHeader,
    pub packets: Vec<Vec<u8>>,
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
    pub initial_garbage_lines: u8,
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
pub struct PktCountdownCancel {
    pub header: PacketHeader,
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
    pub is_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktRoomSnapshot {
    pub header: PacketHeader,
    pub room_code: String,
    pub players: Vec<RoomPlayerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktAddBot {
    pub header: PacketHeader,
    pub temperature: f32,
}

/// Host-only: kick another player from the room (lobby or in-game).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktKickPlayer {
    pub header: PacketHeader,
    pub target_player_id: u8,
}

/// Host-only: remove an AI bot from the room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktRemoveBot {
    pub header: PacketHeader,
    pub target_player_id: u8,
}

/// Final result for a single player in a multiplayer match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingEntry {
    pub player_id: u8,
    pub name: String,
    /// 1 = winner / last survivor; higher = eliminated earlier.
    pub placement: u8,
    pub score: u32,
    pub lines: u32,
    pub survival_ticks: u32,
}

/// Broadcast at global match end: full standings table for the result screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktStandings {
    pub header: PacketHeader,
    pub entries: Vec<StandingEntry>,
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
    pub snapshot: Option<PktStateSnapshot>,
}

/// Resume connection with stored session state. Per D-10.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PktResume {
    pub header: PacketHeader,
    pub socket_id: String,
    pub resume_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bincode_round_trip<T: Serialize + for<'a> Deserialize<'a> + std::fmt::Debug + PartialEq>(
        value: &T,
    ) {
        let bytes = bincode::serialize(value).unwrap();
        let decoded: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(&decoded, value);
    }

    #[test]
    fn test_packet_header_round_trip() {
        let hdr = PacketHeader::new(PacketType::ClientJoin, 0);
        bincode_round_trip(&hdr);
    }

    #[test]
    fn test_game_start_round_trip() {
        let pkt = PktGameStart {
            header: PacketHeader::new(PacketType::GameStart, 0),
            random_seed: 42,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_server_accept_round_trip() {
        let pkt = PktServerAccept {
            header: PacketHeader::new(PacketType::ServerAccept, 0),
            assigned_player_id: 1,
            max_players: 2,
            resume_token: "deadbeefcafef00d".into(),
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
    fn test_batch_round_trip() {
        let inner1 = bincode::serialize(&PktClientJoin {
            header: PacketHeader::new(PacketType::ClientJoin, 0),
        })
        .unwrap();
        let inner2 = bincode::serialize(&PktPlayerReady {
            header: PacketHeader::new(PacketType::PlayerReady, 0),
            ready: true,
        })
        .unwrap();
        let batch = PktBatch {
            header: PacketHeader::new(PacketType::Batch, 0),
            packets: vec![inner1, inner2],
        };
        bincode_round_trip(&batch);
        assert_eq!(batch.packets.len(), 2);
    }

    #[test]
    fn test_create_room_round_trip() {
        let pkt = PktCreateRoom {
            header: PacketHeader::new(PacketType::CreateRoom, 0),
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
            header: PacketHeader::new(PacketType::JoinRoom, 0),
            room_code: "ABCD".into(),
            player_name: "Bob".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_player_ready_round_trip() {
        let pkt = PktPlayerReady {
            header: PacketHeader::new(PacketType::PlayerReady, 1),
            ready: true,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_chat_message_round_trip() {
        let pkt = PktChatMessage {
            header: PacketHeader::new(PacketType::ChatMessage, 1),
            message: "hello".into(),
            timestamp: "2026-06-13T00:00:00Z".into(),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_start_countdown_round_trip() {
        let pkt = PktStartCountdown {
            header: PacketHeader::new(PacketType::StartCountdown, 0),
            remaining_secs: 3,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_countdown_cancel_round_trip() {
        let pkt = PktCountdownCancel {
            header: PacketHeader::new(PacketType::CountdownCancel, 0),
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_host_migrate_round_trip() {
        let pkt = PktHostMigrate {
            header: PacketHeader::new(PacketType::HostMigrate, 0),
            new_host_player_id: 2,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_spectate_switch_round_trip() {
        let pkt = PktSpectateSwitch {
            header: PacketHeader::new(PacketType::SpectateSwitch, 1),
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
            header: PacketHeader::new(PacketType::RoomSnapshot, 0),
            room_code: "ABCD".into(),
            players: vec![RoomPlayerSnapshot {
                player_id: 0,
                name: "Alice".into(),
                ready: true,
                alive: true,
                away: false,
                is_host: true,
                is_bot: false,
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
            header: PacketHeader::new(PacketType::Replay, 0),
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
            header: PacketHeader::new(PacketType::ServerReplay, 0),
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
            header: PacketHeader::new(PacketType::StateHash, 0),
            tick: TickNumber(100),
            hash: 0xDEAD_BEEF,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_state_snapshot_round_trip() {
        let pkt = PktStateSnapshot {
            header: PacketHeader::new(PacketType::StateSnapshot, 0),
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
            header: PacketHeader::new(PacketType::Reconnect, 1),
            last_good_tick: TickNumber(99),
            client_hashes: vec![(TickNumber(0), 0xAAAA), (TickNumber(100), 0xBBBB)],
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_reconnect_ack_round_trip() {
        let pkt = PktReconnectAck {
            header: PacketHeader::new(PacketType::ReconnectAck, 0),
            divergence_tick: TickNumber(99),
            replay_events: vec![],
            snapshot: None,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_pkt_resume_round_trip() {
        let pkt = PktResume {
            header: PacketHeader::new(PacketType::Resume, 1),
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
        assert_eq!(PacketType::IncomingGarbage as u8, 31);
        assert_eq!(PacketType::AddBot as u8, 34);
        assert_eq!(PacketType::CountdownCancel as u8, 35);
    }

    #[test]
    fn test_add_bot_round_trip() {
        let pkt = PktAddBot {
            header: PacketHeader::new(PacketType::AddBot, 0),
            temperature: 0.5,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_kick_remove_packet_types() {
        assert_eq!(PacketType::KickPlayer as u8, 36);
        assert_eq!(PacketType::RemoveBot as u8, 37);
    }

    #[test]
    fn test_kick_player_round_trip() {
        let pkt = PktKickPlayer {
            header: PacketHeader::new(PacketType::KickPlayer, 0),
            target_player_id: 2,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_remove_bot_round_trip() {
        let pkt = PktRemoveBot {
            header: PacketHeader::new(PacketType::RemoveBot, 0),
            target_player_id: 3,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_room_settings_round_trip() {
        let pkt = PktRoomSettings {
            header: PacketHeader::new(PacketType::RoomSettings, 0),
            max_players: 4,
            start_level: 5,
            attack_mult: 1.0,
            garbage_delay_secs: 2,
            allow_hold: false,
            initial_garbage_lines: 3,
        };
        bincode_round_trip(&pkt);
    }

    #[test]
    fn test_standings_round_trip() {
        assert_eq!(PacketType::Standings as u8, 38);
        let pkt = PktStandings {
            header: PacketHeader::new(PacketType::Standings, 0),
            entries: vec![
                StandingEntry {
                    player_id: 0,
                    name: "Alice".into(),
                    placement: 1,
                    score: 12345,
                    lines: 40,
                    survival_ticks: 3600,
                },
                StandingEntry {
                    player_id: 1,
                    name: "Bob".into(),
                    placement: 2,
                    score: 6789,
                    lines: 20,
                    survival_ticks: 1800,
                },
            ],
        };
        bincode_round_trip(&pkt);
        assert_eq!(pkt.entries.len(), 2);
    }
}

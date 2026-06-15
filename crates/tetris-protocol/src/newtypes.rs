use serde::{Deserialize, Serialize};

/// Server tick counter. Wraps u64 to prevent confusion with other integer IDs
/// at compile time. Per D-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TickNumber(pub u64);

/// Client render frame counter. Wraps u64 to distinguish from server ticks.
/// Per D-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameNumber(pub u64);

/// Player index in room (`0..MAX_PLAYERS`). Per D-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerSlot(pub u8);

/// RNG seed for deterministic engine initialization. Per D-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seed(pub i32);

/// Replay message sequence number. Monotonically increasing. Per D-17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MessageSeq(pub u32);

/// Raw key event for deterministic replay. Per D-02.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum KeyAction {
    KeyLeft = 0,
    KeyRight = 1,
    KeySoftDrop = 2,
    KeyHardDrop = 3,
    KeyRotateCW = 4,
    KeyRotateCCW = 5,
    KeyHold = 6,
}

impl KeyAction {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => KeyAction::KeyLeft,
            1 => KeyAction::KeyRight,
            2 => KeyAction::KeySoftDrop,
            3 => KeyAction::KeyHardDrop,
            4 => KeyAction::KeyRotateCW,
            5 => KeyAction::KeyRotateCCW,
            6 => KeyAction::KeyHold,
            _ => KeyAction::KeyLeft,
        }
    }
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
    fn test_tick_number_newtype() {
        assert_eq!(TickNumber(42).0, 42);
        assert_eq!(TickNumber(0), TickNumber(0));
        assert!(TickNumber(10) < TickNumber(20));
    }

    #[test]
    fn test_player_slot_newtype() {
        assert_eq!(PlayerSlot(3).0, 3);
        assert_ne!(PlayerSlot(0), PlayerSlot(1));
    }

    #[test]
    fn test_seed_round_trip() {
        let s = Seed(42);
        bincode_round_trip(&s);
    }

    #[test]
    fn test_message_seq_ord() {
        assert!(MessageSeq(100) < MessageSeq(200));
        assert_eq!(MessageSeq(0), MessageSeq(0));
    }

    #[test]
    fn test_key_action_from_u8() {
        assert_eq!(KeyAction::from_u8(0), KeyAction::KeyLeft);
        assert_eq!(KeyAction::from_u8(6), KeyAction::KeyHold);
        assert_eq!(KeyAction::from_u8(99), KeyAction::KeyLeft);
    }

    #[test]
    fn test_key_action_as_u8() {
        assert_eq!(KeyAction::KeyHardDrop as u8, 3);
        assert_eq!(KeyAction::KeyRotateCW as u8, 4);
    }

    #[test]
    fn test_key_action_round_trip() {
        bincode_round_trip(&KeyAction::KeyLeft);
        bincode_round_trip(&KeyAction::KeyHold);
    }
}

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use crate::error::RelayError;
use tetris_protocol::newtypes::PlayerSlot;
use tetris_protocol::protocol::InputEvent;

pub const RECONNECT_TIMEOUT_SECS: u64 = 3;

/// Zero-sized type state: connection is active and can send input.
pub struct Online;

/// Connection lost, within timeout window.
pub struct Reconnecting;

/// Connection permanently lost (timeout exceeded or explicit leave).
pub struct Dead;

/// Type-state player connection. D-16.
pub struct PlayerConnection<State> {
    pub player_slot: PlayerSlot,
    pub input_tx: Option<tokio::sync::mpsc::Sender<InputEvent>>,
    pub peer_name: String,
    pub disconnected_at: Option<Instant>,
    _state: PhantomData<State>,
}

impl PlayerConnection<Online> {
    pub fn new(
        slot: PlayerSlot,
        input_tx: tokio::sync::mpsc::Sender<InputEvent>,
        name: String,
    ) -> Self {
        Self {
            player_slot: slot,
            input_tx: Some(input_tx),
            peer_name: name,
            disconnected_at: None,
            _state: PhantomData,
        }
    }

    pub fn send_input(&self, event: InputEvent) -> Result<(), RelayError> {
        self.input_tx
            .as_ref()
            .ok_or_else(|| RelayError::WsError("input_tx missing".into()))?
            .try_send(event)
            .map_err(|e| RelayError::WsError(format!("input channel full: {e}")))
    }

    pub fn mark_reconnecting(mut self) -> PlayerConnection<Reconnecting> {
        self.disconnected_at = Some(Instant::now());
        PlayerConnection {
            player_slot: self.player_slot,
            input_tx: self.input_tx,
            peer_name: self.peer_name,
            disconnected_at: self.disconnected_at,
            _state: PhantomData,
        }
    }
}

impl PlayerConnection<Reconnecting> {
    pub fn restore(
        self,
        new_tx: tokio::sync::mpsc::Sender<InputEvent>,
    ) -> PlayerConnection<Online> {
        PlayerConnection {
            player_slot: self.player_slot,
            input_tx: Some(new_tx),
            peer_name: self.peer_name,
            disconnected_at: None,
            _state: PhantomData,
        }
    }

    pub fn mark_dead(mut self) -> PlayerConnection<Dead> {
        self.disconnected_at = Some(Instant::now());
        PlayerConnection {
            player_slot: self.player_slot,
            input_tx: self.input_tx,
            peer_name: self.peer_name,
            disconnected_at: self.disconnected_at,
            _state: PhantomData,
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.disconnected_at.map(|since| Instant::now().duration_since(since))
    }

    pub fn timed_out(&self) -> bool {
        self.elapsed()
            .is_some_and(|d| d >= Duration::from_secs(RECONNECT_TIMEOUT_SECS))
    }
}

impl PlayerConnection<Dead> {
    pub fn into_inner(self) -> (PlayerSlot, String) {
        (self.player_slot, self.peer_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_online_has_send_input() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx, "Alice".into());
        let ev = InputEvent {
            key: tetris_protocol::newtypes::KeyAction::KeyLeft,
            pressed: true,
            tick: tetris_protocol::newtypes::TickNumber(0),
            subframe: 0.0,
        };
        assert!(conn.send_input(ev).is_ok());
    }

    #[test]
    fn test_online_can_transition_to_reconnecting() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx, "Alice".into());
        let reconn = conn.mark_reconnecting();
        assert_eq!(reconn.player_slot, PlayerSlot(0));
    }

    #[test]
    fn test_reconnecting_restore_to_online() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx, "Alice".into());
        let reconn = conn.mark_reconnecting();
        let (new_tx, _new_rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let online = reconn.restore(new_tx);
        assert!(online.disconnected_at.is_none());
    }

    #[test]
    fn test_reconnecting_to_dead() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx, "Alice".into());
        let reconn = conn.mark_reconnecting();
        let dead = reconn.mark_dead();
        let (slot, name) = dead.into_inner();
        assert_eq!(slot, PlayerSlot(0));
        assert_eq!(name, "Alice");
    }

    #[test]
    fn test_reconnecting_not_timed_out_immediately() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx, "Alice".into());
        let reconn = conn.mark_reconnecting();
        assert!(!reconn.timed_out());
    }

    #[test]
    fn test_dead_into_inner() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(3), tx, "Bob".into());
        let dead = conn.mark_reconnecting().mark_dead();
        let (slot, name) = dead.into_inner();
        assert_eq!(slot, PlayerSlot(3));
        assert_eq!(name, "Bob");
    }
}

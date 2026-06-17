use tetris_protocol::newtypes::{PlayerSlot, TickNumber};
use tetris_protocol::protocol::InputEvent;

/// Commands accepted by the authoritative simulation.
pub enum SimCommand {
    AddPlayer {
        slot: PlayerSlot,
    },
    RemovePlayer {
        slot: PlayerSlot,
    },
    PlayerInput {
        slot: PlayerSlot,
        event: InputEvent,
    },
    Reconnect {
        slot: PlayerSlot,
        client_hashes: Vec<(TickNumber, u32)>,
    },
}

/// Serialized protocol packets emitted by the simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimOutbound {
    ToPlayer(PlayerSlot, Vec<u8>),
    Broadcast(Vec<u8>),
}

/// Minimal transport boundary for WS and renet adapters.
pub trait Transport {
    fn send_to(&mut self, slot: PlayerSlot, packet: Vec<u8>);
    fn broadcast(&mut self, packet: Vec<u8>);
}

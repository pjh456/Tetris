#![forbid(unsafe_code)]

//! Transport-agnostic authoritative multiplayer simulation.

pub mod replay;
pub mod sim;
pub mod snapshot;
pub mod transport;

pub use replay::{HashLadder, ReplayBuffer};
pub use sim::{AuthoritativeSim, RoomMode, SimConfig, SimError};
pub use transport::{SimCommand, SimOutbound, Transport};

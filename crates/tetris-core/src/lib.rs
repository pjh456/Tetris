#![forbid(unsafe_code)]

pub mod attack;
pub mod board;
pub mod engine;
pub mod error;
pub mod lockdelay;
pub mod piece;
pub mod rng;
pub mod rules;
pub mod scoring;
pub mod srs;
pub mod state;
pub mod traits;
pub mod types;

pub use attack::{
    AttackResult, COMBO_DMG, NORMAL_DMG, PC_BONUS, TSPIN_DMG, calculate_attack, check_t_spin,
};
pub use board::{Board, ClearResult};
pub use engine::{Action, Engine};
pub use lockdelay::{LOCK_DELAY_MS, LOCK_DELAY_TICKS, LockDelay, LockDelayTicks, MAX_MOVE_RESETS};
pub use piece::{PIECES, PieceDef, Shape};
pub use rng::Lcg;
pub use rules::{can_place, get_ghost_y, hard_drop, lock_piece, try_move, try_rotate};
pub use scoring::ScoreTracker;
pub use srs::SRS;
pub use state::State;
pub use types::{Piece, Rot};

use crate::board::Board;
use crate::types::{Piece, Rot};

#[derive(Debug, Clone)]
pub struct State<const W: usize, const H: usize> {
    pub board: Board<W, H>,
    pub piece: Piece,
    pub rot: Rot,
    pub x: i8,
    pub y: i8,
    pub hold: Piece,
    pub hold_used: bool,
    pub next: [Piece; 5],
    pub rng: u32,
    pub combo: i32,
    pub b2b: bool,
    pub pending_garbage: u8,
    pub last_move_was_rotation: bool,
    pub last_clear_mask: u32,
    pub last_clear_count: u8,
    pub last_harddrop_cols: u16,
    pub last_harddrop_start_y: i8,
    pub last_harddrop_end_y: i8,
    pub last_harddrop_piece: Piece,
    pub last_harddrop_valid: bool,
}

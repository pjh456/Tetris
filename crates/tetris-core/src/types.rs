use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Piece {
    I = 0,
    O = 1,
    T = 2,
    S = 3,
    Z = 4,
    J = 5,
    L = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Rot {
    R0 = 0,
    R90 = 1,
    R180 = 2,
    R270 = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: i8,
    pub y: i8,
}

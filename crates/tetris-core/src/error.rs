use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)] // Reserved for future engine error propagation (Phase 06)
pub enum CoreError {
    #[error("invalid piece type")]
    InvalidPiece,
    #[error("board is full")]
    BoardFull,
}

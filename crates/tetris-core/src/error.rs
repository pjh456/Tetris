use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("invalid piece type")]
    InvalidPiece,
    #[error("board is full")]
    BoardFull,
}

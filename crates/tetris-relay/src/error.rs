use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Room not found: {0}")]
    RoomNotFound(String),
    #[error("Room full: {0}")]
    RoomFull(String),
    #[error("WebSocket error: {0}")]
    WsError(String),
    #[error("Deserialize error: {0}")]
    Decode(String),
}

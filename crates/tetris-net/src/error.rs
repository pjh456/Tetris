use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("bincode encode error: {0}")]
    Encode(String),
    #[error("bincode decode error: {0}")]
    Decode(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("mDNS error: {0}")]
    MdnsError(String),
}

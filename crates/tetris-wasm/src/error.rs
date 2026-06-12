use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("wasm init error: {0}")]
    Init(String),
    #[error("wasm state error: {0}")]
    State(String),
}

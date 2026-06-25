mod decide;
mod mlp;
mod weights;

pub use decide::{decide, decide_seeded, zero_policy};
pub use mlp::{MlpPolicy, softmax_sample, softmax_sample_seeded};
pub use weights::{Layer, LayerNorm, WeightsFile, load_from_slice, load_from_str};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InferError {
    #[error("weights dimensions do not match policy shape")]
    DimMismatch,
    #[error("weights contain non-finite value")]
    NonFinite,
    #[error("weights file exceeds maximum size")]
    TooLarge,
    #[error("weights decode failed: {0}")]
    Decode(String),
}

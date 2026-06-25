use serde::{Deserialize, Serialize};

use crate::{InferError, MlpPolicy};

pub const MAX_WEIGHTS_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightsFile {
    pub input_dim: usize,
    pub output_dim: usize,
    pub activation: String,
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub weight: Vec<Vec<f32>>,
    pub bias: Vec<f32>,
    #[serde(default)]
    pub norm: Option<LayerNorm>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerNorm {
    pub gamma: Vec<f32>,
    pub beta: Vec<f32>,
    #[serde(default = "default_norm_eps")]
    pub eps: f32,
}

fn default_norm_eps() -> f32 {
    1e-5
}

pub fn load_from_slice(bytes: &[u8]) -> Result<MlpPolicy, InferError> {
    if bytes.len() > MAX_WEIGHTS_BYTES {
        return Err(InferError::TooLarge);
    }

    let weights = serde_json::from_slice::<WeightsFile>(bytes)
        .map_err(|err| InferError::Decode(err.to_string()))?;
    MlpPolicy::try_from(weights)
}

pub fn load_from_str(input: &str) -> Result<MlpPolicy, InferError> {
    load_from_slice(input.as_bytes())
}

impl TryFrom<WeightsFile> for MlpPolicy {
    type Error = InferError;

    fn try_from(weights: WeightsFile) -> Result<Self, Self::Error> {
        validate_weights(&weights)?;
        Ok(Self::new(
            weights.input_dim,
            weights.output_dim,
            weights.layers,
        ))
    }
}

fn validate_weights(weights: &WeightsFile) -> Result<(), InferError> {
    if weights.layers.is_empty() || weights.input_dim == 0 || weights.output_dim == 0 {
        return Err(InferError::DimMismatch);
    }

    let mut expected_input_dim = weights.input_dim;
    for layer in &weights.layers {
        validate_layer(layer, expected_input_dim)?;
        expected_input_dim = layer.weight.len();
    }

    if expected_input_dim != weights.output_dim {
        return Err(InferError::DimMismatch);
    }

    Ok(())
}

fn validate_layer(layer: &Layer, input_dim: usize) -> Result<(), InferError> {
    if layer.weight.is_empty() || layer.bias.len() != layer.weight.len() {
        return Err(InferError::DimMismatch);
    }

    for row in &layer.weight {
        if row.len() != input_dim {
            return Err(InferError::DimMismatch);
        }
        if row.iter().any(|value| !value.is_finite()) {
            return Err(InferError::NonFinite);
        }
    }

    if layer.bias.iter().any(|value| !value.is_finite()) {
        return Err(InferError::NonFinite);
    }

    Ok(())
}

use tetris_infer::{InferError, Layer, MlpPolicy, WeightsFile, load_from_str};

fn sample_weights() -> WeightsFile {
    WeightsFile {
        input_dim: 2,
        output_dim: 2,
        activation: "tanh".to_owned(),
        layers: vec![
            Layer {
                weight: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
                bias: vec![0.0, 0.0],
                norm: None,
            },
            Layer {
                weight: vec![vec![1.0, 1.0], vec![1.0, -1.0]],
                bias: vec![0.5, -0.5],
                norm: None,
            },
        ],
    }
}

#[test]
fn weights_round_trip_through_json() -> Result<(), Box<dyn std::error::Error>> {
    let weights = sample_weights();
    let json = serde_json::to_string(&weights)?;
    let decoded = serde_json::from_str::<WeightsFile>(&json)?;
    assert_eq!(decoded, weights);
    Ok(())
}

#[test]
fn load_from_str_accepts_well_formed_weights() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&sample_weights())?;
    let policy = load_from_str(&json)?;
    assert_eq!(policy.input_dim(), 2);
    assert_eq!(policy.output_dim(), 2);
    Ok(())
}

#[test]
fn load_rejects_dim_mismatch() {
    let mut weights = sample_weights();
    weights.layers[0].weight[0].push(1.0);
    let result = MlpPolicy::try_from(weights);
    assert!(matches!(result, Err(InferError::DimMismatch)));
}

#[test]
fn load_rejects_non_finite_values() {
    let mut weights = sample_weights();
    weights.layers[0].bias[0] = f32::INFINITY;
    let result = MlpPolicy::try_from(weights);
    assert!(matches!(result, Err(InferError::NonFinite)));
}

#[test]
fn load_rejects_oversized_bytes() {
    let bytes = vec![b' '; 1_048_577];
    let result = MlpPolicy::load_from_slice(&bytes);
    assert!(matches!(result, Err(InferError::TooLarge)));
}

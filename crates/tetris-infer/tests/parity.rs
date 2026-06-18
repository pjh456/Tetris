use std::{fs, path::Path};

use serde::Deserialize;
use tetris_infer::{WeightsFile, load_from_str};

#[derive(Deserialize)]
struct ParityFixture {
    obs: Vec<f32>,
    logits: Vec<f32>,
    weights: WeightsFile,
}

#[test]
fn parity_fixture_matches_expected_logits() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("tests/fixtures/parity.json");
    if !path.exists() {
        return Ok(());
    }

    let input = fs::read_to_string(path)?;
    let fixture = serde_json::from_str::<ParityFixture>(&input)?;
    let weights = serde_json::to_string(&fixture.weights)?;
    let policy = load_from_str(&weights)?;
    let logits = policy.forward(&fixture.obs);

    assert_eq!(logits.len(), fixture.logits.len());
    for (actual, expected) in logits.iter().zip(&fixture.logits) {
        assert!((actual - expected).abs() < 1.0e-4);
    }

    Ok(())
}

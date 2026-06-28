use tetris_core::engine::Engine;
use tetris_core::rl;
use tetris_core::types::Rot;

use crate::{Layer, MlpPolicy, softmax_sample_seeded};

/// Shared afterstate AI decision: enumerate placements, score each placement's
/// RESULTING board with the value policy (`output_dim == 1`), and pick by softmax
/// over those values. `temperature` is the difficulty knob (→0 = greedy argmax,
/// higher = softer); `seed` keeps independent bots from playing in lockstep.
/// Returns `(col, rot, action_index)` without executing the move or mutating the
/// engine. `None` when there is no legal placement.
pub fn decide(
    engine: &Engine<10, 20>,
    policy: &MlpPolicy,
    temperature: f32,
) -> Option<(i8, Rot, usize)> {
    decide_seeded(engine, policy, temperature, 0x5EED_5EED)
}

/// Like [`decide`] but with an explicit sampling seed.
pub fn decide_seeded(
    engine: &Engine<10, 20>,
    policy: &MlpPolicy,
    temperature: f32,
    seed: u64,
) -> Option<(i8, Rot, usize)> {
    let candidates = rl::afterstate_features(engine);
    if candidates.is_empty() {
        return None;
    }
    // Value of each candidate afterstate (value net is output_dim == 1).
    let values: Vec<f32> = candidates
        .iter()
        .map(|(_, feats)| policy.forward(feats).first().copied().unwrap_or(0.0))
        .collect();
    let pick = softmax_sample_seeded(&values, temperature, seed);
    let action_index = candidates[pick].0;
    let (col, rot) = rl::action_to_placement(action_index)?;
    Some((col, rot, action_index))
}

/// 全零权重策略，供下游 crate 的测试构造确定性 bot。
pub fn zero_policy(input_dim: usize, output_dim: usize) -> MlpPolicy {
    MlpPolicy::new(
        input_dim,
        output_dim,
        vec![Layer {
            weight: vec![vec![0.0; input_dim]; output_dim],
            bias: vec![0.0; output_dim],
            norm: None,
            residual: false,
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_policy() -> MlpPolicy {
        zero_policy(rl::OBS_DIM, 1)
    }

    #[test]
    fn decide_returns_a_legal_afterstate_placement() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let policy = value_policy();
        let decision = decide(&engine, &policy, 0.0);
        assert!(decision.is_some(), "a fresh board has a legal decision");
        if let Some((col, rot, action)) = decision {
            let mask = rl::action_mask(&engine.enumerate_placements());
            assert!(mask[action], "decided action must be legal");
            assert_eq!(rl::action_to_placement(action), Some((col, rot)));
        }
    }

    #[test]
    fn decide_seeded_is_deterministic() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(7);
        let policy = value_policy();
        let first = decide_seeded(&engine, &policy, 1.0, 99);
        let second = decide_seeded(&engine, &policy, 1.0, 99);
        assert_eq!(first, second);
    }
}

use tetris_core::engine::Engine;
use tetris_core::rl;
use tetris_core::types::Rot;

use crate::{Layer, MlpPolicy};

/// 共享 AI 决策：枚举放置 → 编码观测 → 动作掩码 → 策略采样 → 反解放置。
/// 仅选出 `(col, rot, action_index)`，不执行动作、不修改 engine（调用方负责执行/节流）。
/// 无合法放置时返回 `None`。
pub fn decide(
    engine: &Engine<10, 20>,
    policy: &MlpPolicy,
    temperature: f32,
) -> Option<(i8, Rot, usize)> {
    decide_seeded(engine, policy, temperature, 0x5EED_5EED)
}

/// Like [`decide`] but with an explicit sampling seed, so independent bots on
/// identical boards sample different placements instead of playing in lockstep.
pub fn decide_seeded(
    engine: &Engine<10, 20>,
    policy: &MlpPolicy,
    temperature: f32,
    seed: u64,
) -> Option<(i8, Rot, usize)> {
    let placements = engine.enumerate_placements();
    if placements.is_empty() {
        return None;
    }
    let obs = rl::encode_obs(&engine.state);
    let mask = rl::action_mask(&placements);
    let action_index = policy.act_seeded(&obs, &mask, temperature, seed);
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
        }],
    )
}

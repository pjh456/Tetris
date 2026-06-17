use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tetris_core::Engine;

use crate::obs::{ACTION_SPACE_SIZE, action_mask, action_to_placement, encode_obs};
use crate::reward::{RewardConfig, compute_reward};

type PyStepResult<'py> = PyResult<(
    Bound<'py, PyArray1<f32>>,
    f32,
    bool,
    bool,
    Bound<'py, PyDict>,
)>;

#[derive(Debug, Clone)]
pub struct StepOutcome {
    pub obs: Vec<f32>,
    pub reward: f32,
    pub terminated: bool,
    pub truncated: bool,
    pub info: EnvInfo,
}

#[derive(Debug, Clone, Default)]
pub struct EnvInfo {
    pub lines_cleared: u8,
    pub score: u64,
    pub combo: u32,
    pub max_combo: u32,
    pub perfect_clear: bool,
    pub damage: i32,
    pub total_damage: i32,
    pub total_lines: u32,
}

#[pyclass]
pub struct TetrisEnv {
    engine: Engine<10, 20>,
    config: RewardConfig,
    max_steps: u32,
    step_count: u32,
    total_damage: i32,
    total_lines: u32,
}

#[pymethods]
impl TetrisEnv {
    #[new]
    #[pyo3(signature = (max_steps=None, hole_penalty=None, height_penalty=None, bumpiness_penalty=None, well_penalty=None))]
    fn new(
        max_steps: Option<u32>,
        hole_penalty: Option<f32>,
        height_penalty: Option<f32>,
        bumpiness_penalty: Option<f32>,
        well_penalty: Option<f32>,
    ) -> Self {
        Self::new_with_config(
            max_steps.unwrap_or(10_000),
            RewardConfig {
                hole_penalty: hole_penalty.unwrap_or(RewardConfig::default().hole_penalty),
                height_penalty: height_penalty.unwrap_or(RewardConfig::default().height_penalty),
                bumpiness_penalty: bumpiness_penalty
                    .unwrap_or(RewardConfig::default().bumpiness_penalty),
                well_penalty: well_penalty.unwrap_or(RewardConfig::default().well_penalty),
            },
        )
    }

    fn reset<'py>(
        &mut self,
        py: Python<'py>,
        seed: Option<u64>,
    ) -> PyResult<(Bound<'py, PyArray1<f32>>, Bound<'py, PyDict>)> {
        let (obs, info) = self.reset_rust(seed.unwrap_or(0) as u32);
        Ok((PyArray1::from_vec(py, obs), info.to_py_dict(py)?))
    }

    fn step<'py>(&mut self, py: Python<'py>, action: usize) -> PyStepResult<'py> {
        let outcome = self.step_rust(action);
        Ok((
            PyArray1::from_vec(py, outcome.obs),
            outcome.reward,
            outcome.terminated,
            outcome.truncated,
            outcome.info.to_py_dict(py)?,
        ))
    }

    fn obs<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f32>> {
        PyArray1::from_vec(py, self.current_obs())
    }

    fn action_mask<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<bool>> {
        PyArray1::from_vec(py, self.current_action_mask())
    }

    #[staticmethod]
    fn action_space_size() -> usize {
        ACTION_SPACE_SIZE
    }
}

impl TetrisEnv {
    pub fn new_with_config(max_steps: u32, config: RewardConfig) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(0);
        Self {
            engine,
            config,
            max_steps: max_steps.max(1),
            step_count: 0,
            total_damage: 0,
            total_lines: 0,
        }
    }

    pub fn reset_rust(&mut self, seed: u32) -> (Vec<f32>, EnvInfo) {
        self.engine.reset(seed);
        self.step_count = 0;
        self.total_damage = 0;
        self.total_lines = 0;
        (self.current_obs(), self.current_info(0, false, 0))
    }

    pub fn step_rust(&mut self, action: usize) -> StepOutcome {
        let board_before = self.engine.state.board.clone();
        let Some((col, rot)) = action_to_placement(action) else {
            return self.invalid_action_outcome();
        };

        let legal = self
            .engine
            .enumerate_placements()
            .iter()
            .any(|&(legal_col, legal_rot)| legal_col == col && legal_rot == rot);
        if !legal {
            return self.invalid_action_outcome();
        }

        let attack = self.engine.apply_placement(col, rot);
        let lines_cleared = self.engine.state.last_clear_count;
        self.step_count = self.step_count.saturating_add(1);
        self.total_damage = self.total_damage.saturating_add(attack.damage);
        self.total_lines = self.total_lines.saturating_add(u32::from(lines_cleared));

        let reward = compute_reward(
            lines_cleared,
            self.engine.game_over,
            &board_before,
            &self.engine.state.board,
            self.config,
        );
        let truncated = self.step_count >= self.max_steps;
        StepOutcome {
            obs: self.current_obs(),
            reward,
            terminated: self.engine.game_over,
            truncated,
            info: self.current_info(lines_cleared, attack.perfect_clear, attack.damage),
        }
    }

    pub fn current_obs(&self) -> Vec<f32> {
        encode_obs(&self.engine.state)
    }

    pub fn current_action_mask(&self) -> Vec<bool> {
        action_mask(&self.engine.enumerate_placements())
    }

    pub fn state_hash(&self) -> u32 {
        self.engine.state_hash()
    }

    fn invalid_action_outcome(&mut self) -> StepOutcome {
        self.step_count = self.step_count.saturating_add(1);
        StepOutcome {
            obs: self.current_obs(),
            reward: -0.1,
            terminated: self.engine.game_over,
            truncated: self.step_count >= self.max_steps,
            info: self.current_info(0, false, 0),
        }
    }

    fn current_info(&self, lines_cleared: u8, perfect_clear: bool, damage: i32) -> EnvInfo {
        EnvInfo {
            lines_cleared,
            score: self.engine.scorer.score,
            combo: self.engine.scorer.combo,
            max_combo: self.engine.scorer.max_combo,
            perfect_clear,
            damage,
            total_damage: self.total_damage,
            total_lines: self.total_lines,
        }
    }
}

impl EnvInfo {
    fn to_py_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("lines_cleared", self.lines_cleared)?;
        dict.set_item("score", self.score)?;
        dict.set_item("combo", self.combo)?;
        dict.set_item("max_combo", self.max_combo)?;
        dict.set_item("perfect_clear", self.perfect_clear)?;
        dict.set_item("damage", self.damage)?;
        dict.set_item("total_damage", self.total_damage)?;
        dict.set_item("total_lines", self.total_lines)?;
        Ok(dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obs::OBS_DIM;
    use tetris_core::Board;

    fn env() -> TetrisEnv {
        TetrisEnv::new_with_config(10_000, RewardConfig::default())
    }

    #[test]
    fn reset_rust_is_deterministic_for_same_seed() {
        let mut first = env();
        let mut second = env();
        let (first_obs, _) = first.reset_rust(42);
        let (second_obs, _) = second.reset_rust(42);
        assert_eq!(first_obs, second_obs);
    }

    #[test]
    fn current_obs_has_expected_dimension() {
        let mut env = env();
        env.reset_rust(42);
        assert_eq!(env.current_obs().len(), OBS_DIM);
    }

    #[test]
    fn step_rust_returns_transition_data() {
        let mut env = env();
        env.reset_rust(42);
        let action = env
            .current_action_mask()
            .iter()
            .position(|&is_legal| is_legal)
            .unwrap_or(0);
        let outcome = env.step_rust(action);
        assert_eq!(outcome.obs.len(), OBS_DIM);
    }

    #[test]
    fn step_rust_truncates_at_max_steps() {
        let mut env = TetrisEnv::new_with_config(1, RewardConfig::default());
        env.reset_rust(42);
        let outcome = env.step_rust(usize::MAX);
        assert!(outcome.truncated);
    }

    #[test]
    fn deterministic_replay_keeps_hash_equal() {
        let mut first = env();
        let mut second = env();
        first.reset_rust(42);
        second.reset_rust(42);
        for _ in 0..100 {
            let action = first
                .current_action_mask()
                .iter()
                .position(|&is_legal| is_legal)
                .unwrap_or(0);
            first.step_rust(action);
            second.step_rust(action);
        }
        assert_eq!(first.state_hash(), second.state_hash());
    }

    #[test]
    fn info_contains_attack_stats() {
        let mut env = env();
        env.reset_rust(42);
        let outcome = env.step_rust(usize::MAX);
        assert_eq!(outcome.info.total_lines, 0);
    }

    #[test]
    fn board_type_is_accessible_for_reward_snapshots() {
        let board = Board::<10, 20>::new();
        assert_eq!(board.holes(), 0);
    }
}

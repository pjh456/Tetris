use tetris_core::attack::AttackResult;
use tetris_core::engine::Engine;
use tetris_infer::MlpPolicy;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const WEIGHTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../models/weights.json"
));

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WasmAi {
    policy: MlpPolicy,
    engine: Engine<10, 20>,
    temperature: f32,
    outbound_garbage: Vec<(u8, u8)>,
}

impl WasmAi {
    fn from_policy(policy: MlpPolicy, seed: u32) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(seed);
        Self {
            policy,
            engine,
            temperature: 0.0,
            outbound_garbage: Vec::new(),
        }
    }

    fn record_attack(&mut self, attack: AttackResult) {
        if attack.damage > 0 {
            self.outbound_garbage
                .push((attack.damage.min(u8::MAX as i32) as u8, attack.hole_x));
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WasmAi {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(seed: u32) -> Result<WasmAi, String> {
        let policy = MlpPolicy::load_from_str(WEIGHTS)
            .map_err(|err| format!("failed to load embedded AI weights: {err}"))?;
        Ok(Self::from_policy(policy, seed))
    }

    pub fn reset(&mut self, seed: u32) {
        self.engine.reset(seed);
        self.outbound_garbage.clear();
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.temperature = temperature.max(0.0);
    }

    pub fn decide(&mut self) -> u8 {
        let Some((col, rot, action_index)) =
            tetris_infer::decide(&self.engine, &self.policy, self.temperature)
        else {
            return u8::MAX;
        };

        let actions = self.engine.placement_to_actions(col, rot);
        if actions.is_empty() {
            return u8::MAX;
        }
        for action in actions {
            let attack = self.engine.handle_action(action);
            self.record_attack(attack);
        }
        action_index.min(u8::MAX as usize) as u8
    }

    pub fn tick(&mut self, delta_ms: u32) {
        let attack = self.engine.tick(delta_ms);
        self.record_attack(attack);
    }

    pub fn receive_garbage(&mut self, lines: u8, hole_x: u8) {
        // Clamp hole_x into [0, W-1] to keep the garbage row well-formed.
        self.engine.add_pending_garbage(lines, hole_x.min(9), 0);
    }

    pub fn drain_pending_garbage(&mut self) -> Vec<u32> {
        let mut flat = Vec::with_capacity(self.outbound_garbage.len() * 2);
        for (lines, hole_x) in self.outbound_garbage.drain(..) {
            flat.push(u32::from(lines));
            flat.push(u32::from(hole_x));
        }
        flat
    }

    pub fn get_grid_vec(&self) -> Vec<u8> {
        let ghost_y = tetris_core::rules::get_ghost_y(&self.engine.state);
        crate::utils::build_grid(&self.engine.state, ghost_y, self.engine.game_over).to_vec()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_grid(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self.get_grid_vec().as_slice())
    }

    pub fn is_game_over(&self) -> bool {
        self.engine.game_over
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::engine::Action;
    use tetris_core::rl;
    use tetris_core::types::{Piece, Rot};
    use tetris_infer::zero_policy;

    #[test]
    fn wasm_ai_drains_outbound_garbage_after_line_clear() {
        let mut ai = WasmAi::from_policy(zero_policy(rl::OBS_DIM, rl::ACTION_SPACE_SIZE), 42);
        for col in 0..10 {
            if col != 5 && col != 6 {
                ai.engine.state.board.rows[19] |= 1u64 << col;
                ai.engine.state.board.rows[18] |= 1u64 << col;
            }
        }
        ai.engine.state.piece = Piece::O;
        ai.engine.state.rot = Rot::R0;
        ai.engine.state.x = 4;
        ai.engine.state.y = 17;

        let attack = ai.engine.handle_action(Action::HardDrop);
        ai.record_attack(attack);

        assert!(!ai.drain_pending_garbage().is_empty());
    }
}

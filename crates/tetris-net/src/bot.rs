use std::collections::VecDeque;

use tetris_core::engine::{Action, Engine};
use tetris_infer::MlpPolicy;
use tetris_protocol::newtypes::{KeyAction, TickNumber};
use tetris_protocol::protocol::{InputEvent, PacketHeader, PacketType, PktReplay};

pub const BOT_DECIDE_COOLDOWN_TICKS: u32 = 4;

pub struct AiBot {
    policy: MlpPolicy,
    engine: Engine<10, 20>,
    temperature: f32,
    decide_cooldown: u32,
    pending_actions: VecDeque<Action>,
    current_tick: TickNumber,
}

impl AiBot {
    pub fn new(policy: MlpPolicy, seed: u32, temperature: f32) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(seed);
        Self {
            policy,
            engine,
            temperature,
            decide_cooldown: 0,
            pending_actions: VecDeque::new(),
            current_tick: TickNumber(0),
        }
    }

    pub fn next_inputs(&mut self) -> Vec<InputEvent> {
        self.fill_pending_actions();
        let Some(action) = self.pending_actions.pop_front() else {
            self.current_tick.0 = self.current_tick.0.saturating_add(1);
            return Vec::new();
        };

        self.engine.handle_action(action);
        let event = InputEvent {
            key: key_action_for_action(action),
            pressed: true,
            tick: self.current_tick,
            subframe: 0.0,
        };
        self.current_tick.0 = self.current_tick.0.saturating_add(1);
        vec![event]
    }

    pub fn next_replay(&mut self, player_id: u8) -> Option<PktReplay> {
        let events = self.next_inputs();
        if events.is_empty() {
            return None;
        }
        let start_tick = events[0].tick;
        Some(PktReplay {
            header: PacketHeader::new(PacketType::Replay, player_id),
            events,
            start_tick,
        })
    }

    pub fn observe_engine(&mut self, engine: Engine<10, 20>) {
        self.engine = engine;
        self.pending_actions.clear();
        self.decide_cooldown = 0;
    }

    pub fn state_hash(&self) -> u32 {
        self.engine.state_hash()
    }

    fn fill_pending_actions(&mut self) {
        if !self.pending_actions.is_empty() {
            return;
        }
        if self.decide_cooldown > 0 {
            self.decide_cooldown -= 1;
            return;
        }

        let Some((col, rot, _action_index)) =
            tetris_infer::decide(&self.engine, &self.policy, self.temperature)
        else {
            return;
        };

        self.pending_actions = self
            .engine
            .placement_to_actions(col, rot)
            .into_iter()
            .collect();
        if !self.pending_actions.is_empty() {
            self.decide_cooldown = BOT_DECIDE_COOLDOWN_TICKS;
        }
    }
}

fn key_action_for_action(action: Action) -> KeyAction {
    match action {
        Action::MoveLeft => KeyAction::KeyLeft,
        Action::MoveRight => KeyAction::KeyRight,
        Action::SoftDrop => KeyAction::KeySoftDrop,
        Action::HardDrop => KeyAction::KeyHardDrop,
        Action::RotateCW => KeyAction::KeyRotateCW,
        Action::RotateCCW => KeyAction::KeyRotateCCW,
        Action::Hold => KeyAction::KeyHold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::rl;
    use tetris_infer::zero_policy;

    #[test]
    fn bot_emits_thin_input_events() {
        let mut bot = AiBot::new(zero_policy(rl::OBS_DIM, rl::ACTION_SPACE_SIZE), 42, 0.0);
        let inputs = bot.next_inputs();
        assert!(!inputs.is_empty());
        assert!(matches!(
            inputs[0].key,
            KeyAction::KeyLeft
                | KeyAction::KeyRight
                | KeyAction::KeyRotateCW
                | KeyAction::KeyRotateCCW
                | KeyAction::KeyHardDrop
        ));
    }

    #[test]
    fn bot_replay_uses_protocol_packet() {
        let mut bot = AiBot::new(zero_policy(rl::OBS_DIM, rl::ACTION_SPACE_SIZE), 42, 0.0);
        let replay = bot.next_replay(1).unwrap();
        assert_eq!(replay.header.packet_type, PacketType::Replay);
        assert_eq!(replay.header.player_id, 1);
    }
}

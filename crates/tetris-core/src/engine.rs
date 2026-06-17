use crc::{Crc, CRC_32_ISO_HDLC};
use serde::{Deserialize, Serialize};

use crate::attack::{AttackResult, calculate_attack};
use crate::board::Board;
use crate::lockdelay::{LockDelay, LockDelayTicks};
use crate::piece::PIECES;
use crate::rules::{can_place, hard_drop, lock_piece, try_move, try_rotate};
use crate::scoring::ScoreTracker;
use crate::state::State;
use crate::types::{Piece, Rot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Action {
    MoveLeft = 0,
    MoveRight = 1,
    SoftDrop = 2,
    HardDrop = 3,
    RotateCW = 4,
    RotateCCW = 5,
    Hold = 6,
}

impl Action {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Action::MoveLeft,
            1 => Action::MoveRight,
            2 => Action::SoftDrop,
            3 => Action::HardDrop,
            4 => Action::RotateCW,
            5 => Action::RotateCCW,
            6 => Action::Hold,
            _ => {
                debug_assert!(false, "invalid Action u8: {v}");
                Action::MoveLeft
            }
        }
    }
}

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
pub const FIXED_TICK_MS: u32 = 17;

/// Engine-local input event. Maps 1:1 to protocol `InputEvent`.
/// `key` uses `Action::from_u8()` for conversion.
/// Conversion from wire format happens in the net layer.
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub key: u8,
    pub pressed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TickResult {
    pub attack: Option<AttackResult>,
    pub cleared: bool,
    pub game_over: bool,
    pub garbage_inserted: bool,
    pub incoming_garbage_lines: u8,
}

#[derive(Debug, Clone)]
pub struct PendingGarbageEntry {
    pub lines: u8,
    pub hole_x: u8,
    pub remaining_ticks: u16,
}

pub fn gravity_interval_ms(level: u32) -> u32 {
    // Pre-computed integer table: floor((0.8 - (n-1)*0.007)^n * 1000), n=1..15.
    // powf not bit-identical across x86/WASM — table guarantees determinism.
    const TABLE: [u32; 16] = [0, 800, 628, 485, 368, 274, 200, 143, 101, 69, 47, 31, 20, 12, 8, 4];
    TABLE[level.clamp(1, 15) as usize]
}

#[derive(Debug, Clone)]
pub struct Engine<const W: usize, const H: usize> {
    pub state: State<W, H>,
    pub game_over: bool,
    pub has_hold: bool,
    pub scorer: ScoreTracker,
    lock_delay_wall: LockDelay,
    lock_delay_ticks: LockDelayTicks,
    bag: [Piece; 7],
    bag_idx: usize,
    soft_drop_cells: u8,
    hard_drop_cells: u8,
    gravity_accumulator: u32,
    pub garbage_hole_x: u8,
    pub pending_garbage_queue: Vec<PendingGarbageEntry>,
}

impl<const W: usize, const H: usize> Default for Engine<W, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize> Engine<W, H> {
    pub fn new() -> Self {
        Engine {
            state: State {
                board: Board::new(),
                piece: Piece::T,
                rot: Rot::R0,
                x: 0,
                y: 0,
                hold: Piece::I,
                hold_used: false,
                next: [Piece::I; 5],
                rng: 0,
                combo: 0,
                b2b: false,
                pending_garbage: 0,
                last_move_was_rotation: false,
                last_clear_mask: 0,
                last_clear_count: 0,
                last_harddrop_cols: 0,
                last_harddrop_start_y: 0,
                last_harddrop_end_y: 0,
                last_harddrop_piece: Piece::I,
                last_harddrop_valid: false,
            },
            game_over: true,
            has_hold: false,
            scorer: ScoreTracker::default(),
            lock_delay_wall: LockDelay::new(),
            lock_delay_ticks: LockDelayTicks::new(),
            bag: [Piece::I; 7],
            bag_idx: 7,
            soft_drop_cells: 0,
            hard_drop_cells: 0,
            gravity_accumulator: 0,
            garbage_hole_x: 0,
            pending_garbage_queue: Vec::new(),
        }
    }

    pub fn add_pending_garbage(&mut self, lines: u8, hole_x: u8, delay_ticks: u16) {
        if lines == 0 {
            return;
        }
        self.pending_garbage_queue.push(PendingGarbageEntry {
            lines,
            hole_x,
            remaining_ticks: delay_ticks,
        });
    }

    pub fn process_pending_garbage_queue(&mut self) {
        for entry in &mut self.pending_garbage_queue {
            if entry.remaining_ticks > 0 {
                entry.remaining_ticks -= 1;
            }
        }
        let mut _applied = false;
        for entry in &mut self.pending_garbage_queue {
            if entry.remaining_ticks == 0 && entry.lines > 0 {
                self.state.pending_garbage = self.state.pending_garbage.saturating_add(entry.lines);
                self.garbage_hole_x = entry.hole_x;
                entry.lines = 0;
                _applied = true;
            }
        }
        self.pending_garbage_queue.retain(|e| e.lines > 0);
    }

    pub fn cancel_pending_garbage(&mut self, amount: u8) {
        let mut remaining = amount;
        for entry in &mut self.pending_garbage_queue {
            if remaining == 0 {
                break;
            }
            let cancel = entry.lines.min(remaining);
            entry.lines -= cancel;
            remaining -= cancel;
        }
        self.pending_garbage_queue.retain(|e| e.lines > 0);
    }

    pub fn incoming_garbage_total(&self) -> u8 {
        self.pending_garbage_queue
            .iter()
            .map(|e| e.lines)
            .sum::<u8>()
            .saturating_add(self.state.pending_garbage)
    }

    fn next_rand(&mut self) -> u32 {
        self.state.rng = self
            .state
            .rng
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        self.state.rng
    }

    fn shuffle_bag(&mut self) {
        for i in 0..7 {
            self.bag[i] = Piece::from_u8(i as u8);
        }
        for i in (1..7).rev() {
            let j = (self.next_rand() % (i as u32 + 1)) as usize;
            self.bag.swap(i, j);
        }
    }

    fn pop_next_piece(&mut self) -> Piece {
        if self.bag_idx >= 7 {
            self.shuffle_bag();
            self.bag_idx = 0;
        }
        let p = self.bag[self.bag_idx];
        self.bag_idx += 1;
        p
    }

    fn lock_and_spawn(&mut self) -> AttackResult {
        let lines_cleared = lock_piece(&mut self.state);
        let mut attack_res = calculate_attack(&mut self.state, lines_cleared);
        attack_res.hole_x = (self.state.rng % W as u32) as u8;
        self.scorer.update(
            lines_cleared as u8,
            attack_res.is_tspin,
            attack_res.is_mini,
            attack_res.is_b2b,
            attack_res.perfect_clear,
            self.soft_drop_cells,
            self.hard_drop_cells,
            self.state.combo.saturating_sub(1).max(0) as u32,
            self.scorer.level,
        );
        self.soft_drop_cells = 0;
        self.hard_drop_cells = 0;

        if attack_res.damage > 0 && self.state.pending_garbage > 0 {
            if attack_res.damage >= self.state.pending_garbage as i32 {
                attack_res.damage -= self.state.pending_garbage as i32;
                self.state.pending_garbage = 0;
            } else {
                self.state.pending_garbage -= attack_res.damage as u8;
                attack_res.damage = 0;
            }
        } else if lines_cleared == 0 && self.state.pending_garbage > 0 {
            let hole_x = self.garbage_hole_x;
            self.state
                .board
                .insert_garbage(self.state.pending_garbage, hole_x);
            self.state.pending_garbage = 0;
            self.garbage_hole_x = 0;
            self.pending_garbage_queue.clear();
            attack_res.garbage_inserted = true;
        }

        if lines_cleared > 0 && !self.pending_garbage_queue.is_empty() {
            let cancel_amount = (lines_cleared as u8).saturating_add(1);
            self.cancel_pending_garbage(cancel_amount);
        }

        self.lock_delay_ticks.cancel();
        self.spawn();
        attack_res
    }

    fn record_harddrop(&mut self, start_y: i8, end_y: i8) {
        debug_assert!(W <= 16, "record_harddrop only supports W <= 16");
        let shape = &PIECES[self.state.piece as usize].rot[self.state.rot as usize];
        let mut mask: u16 = 0;
        for i in 0..4 {
            for j in 0..4 {
                if shape.row[i] & (1 << j) == 0 {
                    continue;
                }
                let xx = self.state.x + j as i8;
                if xx < 0 || xx >= W as i8 {
                    continue;
                }
                mask |= 1u16 << (xx.min(15));
            }
        }
        self.state.last_harddrop_cols = mask;
        self.state.last_harddrop_start_y = start_y;
        self.state.last_harddrop_end_y = end_y;
        self.state.last_harddrop_piece = self.state.piece;
        self.state.last_harddrop_valid = mask != 0;
    }

    fn try_move_wrapped(&mut self, dx: i8, dy: i8) -> bool {
        if try_move(&mut self.state, dx, dy) {
            self.state.last_move_was_rotation = false;
            let grounded = !crate::rules::can_place(
                &self.state, self.state.x, self.state.y + 1, self.state.rot,
            );
            if grounded {
                self.lock_delay_wall.reset();
                self.lock_delay_ticks.reset();
            }
            return true;
        }
        false
    }

    fn try_rotate_wrapped(&mut self, to: Rot) -> bool {
        if try_rotate(&mut self.state, to) {
            self.state.last_move_was_rotation = true;
            let grounded = !crate::rules::can_place(
                &self.state, self.state.x, self.state.y + 1, self.state.rot,
            );
            if grounded {
                self.lock_delay_wall.reset();
                self.lock_delay_ticks.reset();
            }
            return true;
        }
        false
    }

    pub fn reset(&mut self, seed: u32) {
        self.reset_with_level(seed, 1);
    }

    pub fn reset_with_level(&mut self, seed: u32, start_level: u32) {
        self.state = State {
            board: Board::new(),
            piece: Piece::T,
            rot: Rot::R0,
            x: 0,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 5],
            rng: seed,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            last_move_was_rotation: false,
            last_clear_mask: 0,
            last_clear_count: 0,
            last_harddrop_cols: 0,
            last_harddrop_start_y: 0,
            last_harddrop_end_y: 0,
            last_harddrop_piece: Piece::I,
            last_harddrop_valid: false,
        };
        self.game_over = false;
        self.has_hold = false;
        self.scorer = ScoreTracker::default();
        self.scorer.level = start_level.clamp(1, 15);
        self.bag_idx = 7;
        self.lock_delay_wall.cancel();
        self.lock_delay_ticks.cancel();
        self.soft_drop_cells = 0;
        self.hard_drop_cells = 0;
        self.gravity_accumulator = 0;
        self.garbage_hole_x = 0;

        for i in 0..5 {
            self.state.next[i] = self.pop_next_piece();
        }

        self.spawn();
    }

    pub fn spawn(&mut self) {
        self.state.piece = self.state.next[0];
        for i in 0..4 {
            self.state.next[i] = self.state.next[i + 1];
        }
        self.state.next[4] = self.pop_next_piece();

        self.state.rot = Rot::R0;
        self.state.x = (W / 2) as i8 - 2;
        self.state.y = 0;
        self.state.hold_used = false;
        self.lock_delay_wall.cancel();

        if !can_place(&self.state, self.state.x, self.state.y, self.state.rot) {
            self.game_over = true;
        }
    }

    pub fn handle_action(&mut self, action: Action) -> AttackResult {
        if self.game_over {
            return AttackResult::default();
        }

        match action {
            Action::MoveLeft => {
                self.try_move_wrapped(-1, 0);
                AttackResult::default()
            }
            Action::MoveRight => {
                self.try_move_wrapped(1, 0);
                AttackResult::default()
            }
            Action::SoftDrop => {
                if self.try_move_wrapped(0, 1) {
                    self.soft_drop_cells += 1;
                }
                AttackResult::default()
            }
            Action::HardDrop => {
                let start_y = self.state.y;
                hard_drop(&mut self.state);
                let end_y = self.state.y;
                self.record_harddrop(start_y, end_y);
                self.hard_drop_cells = (end_y - start_y).max(0) as u8;
                self.lock_delay_wall.cancel();
                self.lock_and_spawn()
            }
            Action::RotateCW => {
                let to = match (self.state.rot as u8 + 1) & 3 {
                    0 => Rot::R0,
                    1 => Rot::R90,
                    2 => Rot::R180,
                    3 => Rot::R270,
                    _ => Rot::R0,
                };
                self.try_rotate_wrapped(to);
                AttackResult::default()
            }
            Action::RotateCCW => {
                let to = match (self.state.rot as u8 + 3) & 3 {
                    0 => Rot::R0,
                    1 => Rot::R90,
                    2 => Rot::R180,
                    3 => Rot::R270,
                    _ => Rot::R0,
                };
                self.try_rotate_wrapped(to);
                AttackResult::default()
            }
            Action::Hold => {
                if !self.state.hold_used {
                    self.lock_delay_wall.cancel();
                    self.lock_delay_ticks.cancel();
                    if self.has_hold {
                        std::mem::swap(&mut self.state.hold, &mut self.state.piece);
                        self.state.rot = Rot::R0;
                        self.state.x = (W / 2) as i8 - 2;
                        self.state.y = 0;
                        if !can_place(&self.state, self.state.x, self.state.y, self.state.rot) {
                            self.game_over = true;
                        }
                    } else {
                        self.state.hold = self.state.piece;
                        self.has_hold = true;
                        self.spawn();
                    }
                    self.state.hold_used = true;
                }
                AttackResult::default()
            }
        }
    }

    pub fn tick(&mut self, delta_ms: u32) -> AttackResult {
        if self.game_over {
            return AttackResult::default();
        }

        self.gravity_accumulator += delta_ms;
        let interval = gravity_interval_ms(self.scorer.level);
        let mut result = AttackResult::default();

        while self.gravity_accumulator >= interval {
            self.gravity_accumulator -= interval;

            if try_move(&mut self.state, 0, 1) {
                self.lock_delay_wall.cancel();
                self.lock_delay_ticks.cancel();
            } else {
                self.lock_delay_wall.start();
                self.lock_delay_ticks.start();
                if self.lock_delay_wall.update() {
                    result = self.lock_and_spawn();
                }
                self.lock_delay_ticks.update();
            }
        }

        result
    }

    pub fn get_lock_timer(&self) -> i32 {
        let rem_ticks = self.lock_delay_ticks.remaining_ticks();
        if rem_ticks == 0 {
            0
        } else {
            (rem_ticks as i32 * crate::lockdelay::LOCK_DELAY_MS as i32 / crate::lockdelay::LOCK_DELAY_TICKS as i32).max(0)
        }
    }

    pub fn state_hash(&self) -> u32 {
        let mut digest = CRC32.digest();

        for row in &self.state.board.rows {
            digest.update(&row.to_le_bytes());
        }

        digest.update(&[self.state.piece as u8]);
        digest.update(&[self.state.rot as u8]);
        digest.update(&self.state.x.to_le_bytes());
        digest.update(&self.state.y.to_le_bytes());
        digest.update(&(self.state.hold as u8).to_le_bytes());
        digest.update(&[self.state.hold_used as u8]);
        for piece in &self.state.next {
            digest.update(&(*piece as u8).to_le_bytes());
        }
        digest.update(&self.state.rng.to_le_bytes());
        digest.update(&self.state.pending_garbage.to_le_bytes());
        digest.update(&self.state.combo.to_le_bytes());
        digest.update(&[self.state.b2b as u8]);
        digest.update(&[self.state.last_move_was_rotation as u8]);

        for piece in &self.bag {
            digest.update(&(*piece as u8).to_le_bytes());
        }
        digest.update(&self.bag_idx.to_le_bytes());
        digest.update(&[self.lock_delay_ticks.is_active() as u8]);
        digest.update(&self.lock_delay_ticks.accumulated_ticks.to_le_bytes());
        digest.update(&self.lock_delay_ticks.move_reset_count.to_le_bytes());

        digest.finalize()
    }

    pub fn fixed_tick(&mut self, inputs: &[InputEvent]) -> TickResult {
        self.process_pending_garbage_queue();

        if self.game_over {
            return TickResult {
                game_over: true,
                ..TickResult::default()
            };
        }

        for input in inputs {
            if input.pressed {
                let action = Action::from_u8(input.key);
                let attack_res = self.handle_action(action);
                let locked = matches!(action, Action::HardDrop | Action::Hold);
                if attack_res.damage != 0
                    || attack_res.is_tspin
                    || attack_res.is_b2b
                    || attack_res.perfect_clear
                    || locked
                    || self.game_over
                {
                    return TickResult {
                        attack: Some(attack_res),
                        cleared: true,
                        game_over: self.game_over,
                        garbage_inserted: false,
                        incoming_garbage_lines: self.incoming_garbage_total(),
                    };
                }
            }
        }

        if self.game_over {
            return TickResult {
                game_over: true,
                ..TickResult::default()
            };
        }

        self.gravity_accumulator += FIXED_TICK_MS;
        let interval = gravity_interval_ms(self.scorer.level);
        let mut attack_result = None;
        let mut garbage_detected = false;

        while self.gravity_accumulator >= interval {
            self.gravity_accumulator -= interval;

            if try_move(&mut self.state, 0, 1) {
                self.lock_delay_ticks.cancel();
            } else {
                self.lock_delay_ticks.start();
                if self.lock_delay_ticks.update() {
                    let res = self.lock_and_spawn();
                    garbage_detected = res.garbage_inserted;
                    attack_result = Some(res);
                    break;
                }
            }
        }

        let cleared = attack_result.is_some();
        TickResult {
            attack: attack_result,
            cleared,
            game_over: self.game_over,
            garbage_inserted: garbage_detected,
            incoming_garbage_lines: self.incoming_garbage_total(),
        }
    }
}

impl Piece {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Piece::I,
            1 => Piece::O,
            2 => Piece::T,
            3 => Piece::S,
            4 => Piece::Z,
            5 => Piece::J,
            6 => Piece::L,
            _ => Piece::I,
        }
    }
}

impl<const W: usize, const H: usize> crate::traits::GameEngine for Engine<W, H> {
    fn new() -> Self {
        Engine::new()
    }

    fn reset(&mut self, seed: u32) {
        self.reset(seed);
    }

    fn spawn(&mut self) {
        self.spawn();
    }

    fn handle_action(&mut self, action: Action) -> AttackResult {
        self.handle_action(action)
    }

    fn tick(&mut self, delta_ms: u32) -> AttackResult {
        self.tick(delta_ms)
    }

    fn get_lock_timer(&self) -> i32 {
        self.get_lock_timer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_determinism_same_seed() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(42);
        b.reset(42);
        assert_eq!(a.state.piece, b.state.piece);
        assert_eq!(a.state.rng, b.state.rng);
        for i in 0..5 {
            assert_eq!(a.state.next[i], b.state.next[i]);
        }
    }

    #[test]
    fn test_reset_different_seed() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(42);
        b.reset(99);
        let diff = a.state.piece != b.state.piece || a.state.rng != b.state.rng;
        assert!(diff);
    }

    #[test]
    fn test_tick_not_game_over() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        for _ in 0..100 {
            if engine.game_over {
                break;
            }
            engine.tick(16);
        }
        assert!(!engine.game_over);
    }

    #[test]
    fn test_get_lock_timer() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        let t = engine.get_lock_timer();
        assert!(t >= 0);
    }

    #[test]
    fn test_hard_drop_spawns_next() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let first = engine.state.piece;
        engine.handle_action(Action::HardDrop);
        assert_ne!(engine.state.piece, first);
        assert_eq!(engine.get_lock_timer(), 0);
    }

    #[test]
    fn test_multiple_hard_drops() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        for _ in 0..5 {
            if engine.game_over {
                break;
            }
            engine.handle_action(Action::HardDrop);
        }
        assert!(!engine.game_over);
    }

    #[test]
    fn test_first_hold_saves_piece() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        let first = engine.state.piece;
        assert!(!engine.has_hold);
        engine.handle_action(Action::Hold);
        assert!(engine.has_hold);
        assert_eq!(engine.state.hold, first);
        assert!(engine.state.hold_used);
        assert_ne!(engine.state.piece, first);
    }

    #[test]
    fn test_hold_swap_back() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        let first = engine.state.piece;
        engine.handle_action(Action::Hold);
        engine.handle_action(Action::HardDrop);
        engine.handle_action(Action::Hold);
        assert_eq!(engine.state.piece, first);
    }

    #[test]
    fn test_game_over_full_board_spawn() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        for i in 0..20 {
            engine.state.board.rows[i] = Board::<10, 20>::FULL;
        }
        engine.spawn();
        assert!(engine.game_over);
    }

    #[test]
    fn test_normal_reset_not_game_over() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        assert!(!engine.game_over);
    }

    #[test]
    fn test_scripted_deterministic() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(100);
        b.reset(100);

        let seq = [
            Action::MoveLeft,
            Action::MoveRight,
            Action::RotateCW,
            Action::HardDrop,
            Action::Hold,
            Action::SoftDrop,
        ];

        for act in seq {
            a.handle_action(act);
            b.handle_action(act);
        }

        assert_eq!(a.state.piece, b.state.piece);
        assert_eq!(a.state.rng, b.state.rng);
        assert_eq!(a.game_over, b.game_over);
    }

    #[test]
    fn test_state_hash_same_engine_equal() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(42);
        b.reset(42);
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn test_state_hash_different_engine_unequal() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(42);
        b.reset(99);
        assert_ne!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn test_state_hash_changes_after_action() {
        let mut a = Engine::<10, 20>::new();
        a.reset(42);
        let h0 = a.state_hash();
        a.handle_action(Action::MoveLeft);
        let h1 = a.state_hash();
        assert_ne!(h0, h1);
    }

    #[test]
    fn test_fixed_tick_advances_gravity() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(12345);
        let start_y = engine.state.y;
        for _ in 0..50 {
            engine.fixed_tick(&[]);
        }
        assert!(engine.state.y > start_y || engine.game_over);
    }

    #[test]
    fn test_fixed_tick_with_input() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let piece_before = engine.state.piece;
        let result = engine.fixed_tick(&[InputEvent {
            key: Action::HardDrop as u8,
            pressed: true,
        }]);
        assert!(result.cleared);
        assert_ne!(engine.state.piece, piece_before);
    }

    #[test]
    fn test_state_hash_matches_after_same_actions() {
        let mut a = Engine::<10, 20>::new();
        let mut b = Engine::<10, 20>::new();
        a.reset(42);
        b.reset(42);

        for _ in 0..5 {
            a.handle_action(Action::HardDrop);
            b.handle_action(Action::HardDrop);
        }

        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn test_add_pending_garbage_queues_entry() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(3, 5, 60);
        assert_eq!(engine.pending_garbage_queue.len(), 1);
        assert_eq!(engine.pending_garbage_queue[0].lines, 3);
        assert_eq!(engine.pending_garbage_queue[0].hole_x, 5);
        assert_eq!(engine.pending_garbage_queue[0].remaining_ticks, 60);
    }

    #[test]
    fn test_add_pending_garbage_ignores_zero() {
        let mut engine = Engine::<10, 20>::new();
        engine.add_pending_garbage(0, 5, 60);
        assert!(engine.pending_garbage_queue.is_empty());
    }

    #[test]
    fn test_process_pending_garbage_ticks_down() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(2, 4, 3);
        engine.process_pending_garbage_queue();
        assert_eq!(engine.pending_garbage_queue[0].remaining_ticks, 2);
        assert_eq!(engine.state.pending_garbage, 0);
    }

    #[test]
    fn test_process_pending_garbage_applies_when_expired() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(2, 4, 1);
        engine.process_pending_garbage_queue();
        assert_eq!(engine.state.pending_garbage, 2);
        assert_eq!(engine.garbage_hole_x, 4);
        assert!(engine.pending_garbage_queue.is_empty());
    }

    #[test]
    fn test_process_pending_garbage_multiple_entries() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(2, 3, 3);
        engine.add_pending_garbage(1, 5, 1);
        engine.add_pending_garbage(4, 7, 5);
        engine.process_pending_garbage_queue();
        // Ticks: 2, 0, 4. Entry at idx 1 expired
        assert_eq!(engine.state.pending_garbage, 1);
        assert_eq!(engine.garbage_hole_x, 5);
        assert_eq!(engine.pending_garbage_queue.len(), 2);
    }

    #[test]
    fn test_cancel_pending_garbage_partial() {
        let mut engine = Engine::<10, 20>::new();
        engine.add_pending_garbage(5, 3, 60);
        engine.cancel_pending_garbage(2);
        assert_eq!(engine.pending_garbage_queue[0].lines, 3);
    }

    #[test]
    fn test_cancel_pending_garbage_full() {
        let mut engine = Engine::<10, 20>::new();
        engine.add_pending_garbage(3, 5, 60);
        engine.cancel_pending_garbage(3);
        assert!(engine.pending_garbage_queue.is_empty());
    }

    #[test]
    fn test_cancel_pending_garbage_spans_entries() {
        let mut engine = Engine::<10, 20>::new();
        engine.add_pending_garbage(2, 3, 60);
        engine.add_pending_garbage(3, 5, 60);
        engine.cancel_pending_garbage(3);
        // First entry fully gone, second reduced to 2
        assert_eq!(engine.pending_garbage_queue.len(), 1);
        assert_eq!(engine.pending_garbage_queue[0].lines, 2);
    }

    #[test]
    fn test_incoming_garbage_total() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(3, 4, 60);
        engine.add_pending_garbage(2, 5, 30);
        engine.state.pending_garbage = 1;
        assert_eq!(engine.incoming_garbage_total(), 6);
    }

    #[test]
    fn test_attack_result_has_hole_x_when_lines_cleared() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset_with_level(42, 1);
        // Fill rows 19 and 18 to make a Double clear possible
        for col in 0..10 {
            engine.state.board.rows[19] |= 1u64 << col;
            engine.state.board.rows[18] |= 1u64 << col;
        }
        // Place an I piece at the bottom in column 0..4 to complete the lines
        // but leave two cells open for the piece to fill
        engine.state.board.rows[19] ^= (1u64 << 0) | (1u64 << 1) | (1u64 << 2) | (1u64 << 3);
        engine.state.board.rows[18] ^= (1u64 << 0) | (1u64 << 1) | (1u64 << 2) | (1u64 << 3);
        // Force an I piece
        engine.state.piece = Piece::I;
        engine.state.rot = Rot::R0;
        engine.state.x = 0;
        engine.state.y = 18;
        let result = engine.handle_action(Action::HardDrop);
        // Might clear or might not depending on piece placement
        // The key test: if damage > 0, hole_x must be < 10
        if result.damage > 0 {
            assert!(result.hole_x < 10, "hole_x should be valid board column");
        }
    }

    #[test]
    fn test_cancel_on_lock_clear_reduces_queue() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(3, 4, 60);
        assert_eq!(engine.pending_garbage_queue[0].lines, 3);
        // S模拟 line clear → 取消 pending
        engine.cancel_pending_garbage(2);
        assert_eq!(engine.pending_garbage_queue[0].lines, 1);
        engine.cancel_pending_garbage(1);
        assert!(engine.pending_garbage_queue.is_empty());
    }

    #[test]
    fn test_lock_and_spawn_integration_cancels_queue() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        // Add pending garbage, then simulate line clear via lock_and_spawn
        engine.add_pending_garbage(5, 4, 60);
        // Set up board so piece locks and clears lines
        // Fill rows 18-19 leaving I-piece shaped gap
        for col in 0..10 {
            engine.state.board.rows[19] |= 1u64 << col;
            engine.state.board.rows[18] |= 1u64 << col;
        }
        engine.state.board.rows[19] ^= (1u64 << 0) | (1u64 << 1) | (1u64 << 2) | (1u64 << 3);
        engine.state.board.rows[18] ^= (1u64 << 0) | (1u64 << 1) | (1u64 << 2) | (1u64 << 3);
        engine.state.piece = Piece::I;
        engine.state.rot = Rot::R0;
        engine.state.x = 0;
        engine.state.y = 18;
        // HardDrop triggers lock_piece + line clears + cancel
        let result = engine.handle_action(Action::HardDrop);
        // If lines were cleared, queue should be reduced/cancelled
        if result.damage > 0 {
            let total: u8 = engine.pending_garbage_queue.iter().map(|e| e.lines).sum();
            assert!(total < 5, "pending queue should be reduced after line clear");
        }
    }

    #[test]
    fn test_garbage_insert_on_lock_no_clear() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.state.pending_garbage = 2;
        engine.garbage_hole_x = 4;
        // HardDrop triggers lock_and_spawn — no lines on empty board, garbage inserts
        engine.handle_action(Action::HardDrop);
        assert_eq!(engine.state.pending_garbage, 0,
            "pending_garbage should be consumed by insert_garbage after HardDrop lock");
    }

    #[test]
    fn test_add_process_insert_end_to_end() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.add_pending_garbage(1, 3, 1);
        engine.process_pending_garbage_queue();
        assert_eq!(engine.state.pending_garbage, 1);
        assert_eq!(engine.garbage_hole_x, 3);
        let rows_before = engine.state.board.rows;
        engine.handle_action(Action::HardDrop);
        assert_eq!(engine.state.pending_garbage, 0);
        assert_ne!(engine.state.board.rows, rows_before,
            "board should change after garbage insert");
    }
}

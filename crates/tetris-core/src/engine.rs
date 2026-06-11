use serde::{Deserialize, Serialize};

use crate::attack::{AttackResult, calculate_attack};
use crate::board::Board;
use crate::lockdelay::LockDelay;
use crate::piece::PIECES;
use crate::rules::{can_place, hard_drop, lock_piece, try_move, try_rotate};
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
            _ => Action::MoveLeft,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lcg(pub u32);

impl Lcg {
    pub fn new(seed: u32) -> Self {
        Lcg(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Engine<const W: usize, const H: usize> {
    pub state: State<W, H>,
    pub game_over: bool,
    pub has_hold: bool,
    lock_delay: LockDelay,
    bag: [Piece; 7],
    bag_idx: usize,
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
            lock_delay: LockDelay::new(),
            bag: [Piece::I; 7],
            bag_idx: 7,
        }
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

        if attack_res.damage > 0 && self.state.pending_garbage > 0 {
            if attack_res.damage >= self.state.pending_garbage as i32 {
                attack_res.damage -= self.state.pending_garbage as i32;
                self.state.pending_garbage = 0;
            } else {
                self.state.pending_garbage -= attack_res.damage as u8;
                attack_res.damage = 0;
            }
        } else if lines_cleared == 0 && self.state.pending_garbage > 0 {
            let hole_x = (self.next_rand() % W as u32) as u8;
            self.state
                .board
                .insert_garbage(self.state.pending_garbage, hole_x);
            self.state.pending_garbage = 0;
        }

        self.spawn();
        attack_res
    }

    fn record_harddrop(&mut self, start_y: i8, end_y: i8) {
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
                mask |= 1u16 << xx;
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
            if self.lock_delay.move_reset_count < crate::lockdelay::MAX_MOVE_RESETS {
                self.lock_delay.reset();
            }
            return true;
        }
        false
    }

    fn try_rotate_wrapped(&mut self, to: Rot) -> bool {
        if try_rotate(&mut self.state, to) {
            self.state.last_move_was_rotation = true;
            if self.lock_delay.move_reset_count < crate::lockdelay::MAX_MOVE_RESETS {
                self.lock_delay.reset();
            }
            return true;
        }
        false
    }

    pub fn reset(&mut self, seed: u32) {
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
        self.bag_idx = 7;
        self.lock_delay.cancel();

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
        self.lock_delay.cancel();

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
                self.try_move_wrapped(0, 1);
                AttackResult::default()
            }
            Action::HardDrop => {
                let start_y = self.state.y;
                hard_drop(&mut self.state);
                let end_y = self.state.y;
                self.record_harddrop(start_y, end_y);
                self.lock_delay.cancel();
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
                    self.lock_delay.cancel();
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

    pub fn tick(&mut self) -> AttackResult {
        if self.game_over {
            return AttackResult::default();
        }

        if try_move(&mut self.state, 0, 1) {
            self.lock_delay.cancel();
            return AttackResult::default();
        }

        self.lock_delay.start();
        if self.lock_delay.update() {
            return self.lock_and_spawn();
        }

        AttackResult::default()
    }

    pub fn get_lock_timer(&self) -> i32 {
        self.lock_delay.remaining_ms()
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
            engine.tick();
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
}

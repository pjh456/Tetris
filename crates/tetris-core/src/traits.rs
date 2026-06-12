use crate::attack::AttackResult;
use crate::board::ClearResult;
use crate::engine::Action;

pub trait TetrisBoard {
    const FULL: u64;

    fn new() -> Self;
    fn collide(&self, y: u8, mask: u64) -> bool;
    fn place(&mut self, y: u8, mask: u64);
    fn full(&self, y: u8) -> bool;
    fn is_empty(&self) -> bool;
    fn clear_lines(&mut self) -> ClearResult;
    fn insert_garbage(&mut self, lines: u8, hole_x: u8);
}

pub trait GameEngine {
    fn new() -> Self;
    fn reset(&mut self, seed: u32);
    fn spawn(&mut self);
    fn handle_action(&mut self, action: Action) -> AttackResult;
    fn tick(&mut self) -> AttackResult;
    fn get_lock_timer(&self) -> i32;
}

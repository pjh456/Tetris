use instant::Instant;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LockDelay {
    active: bool,
    pub move_reset_count: u32,
    lock_deadline: Instant,
}

pub const MAX_MOVE_RESETS: u32 = 15;
pub const LOCK_DELAY_MS: u64 = 500;

impl LockDelay {
    pub fn new() -> Self {
        LockDelay {
            active: false,
            move_reset_count: 0,
            lock_deadline: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        if !self.active {
            self.active = true;
            self.lock_deadline = Instant::now() + Duration::from_millis(LOCK_DELAY_MS);
        }
    }

    pub fn reset(&mut self) {
        self.active = true;
        self.move_reset_count += 1;
        self.lock_deadline = Instant::now() + Duration::from_millis(LOCK_DELAY_MS);
    }

    pub fn update(&mut self) -> bool {
        if !self.active {
            return false;
        }

        if Instant::now() >= self.lock_deadline {
            self.active = false;
            self.move_reset_count = 0;
            return true;
        }
        false
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.move_reset_count = 0;
    }

    pub fn remaining_ms(&self) -> i32 {
        if !self.active {
            return 0;
        }
        let elapsed = self.lock_deadline.saturating_duration_since(Instant::now());
        elapsed.as_millis() as i32
    }
}

impl Default for LockDelay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_active_by_default() {
        let ld = LockDelay::new();
        assert!(!ld.active);
        assert_eq!(ld.move_reset_count, 0);
        let mut ld = ld;
        assert!(!ld.update());
        assert_eq!(ld.remaining_ms(), 0);
    }

    #[test]
    fn test_start_activates_timer() {
        let mut ld = LockDelay::new();
        ld.start();
        assert!(ld.active);
        let rem = ld.remaining_ms();
        assert!(rem > 0);
        assert!(rem <= LOCK_DELAY_MS as i32);
        assert!(!ld.update());
    }

    #[test]
    fn test_start_is_idempotent() {
        let mut ld = LockDelay::new();
        ld.start();
        ld.start();
        assert!(ld.active);
    }

    #[test]
    fn test_reset_increments_counter() {
        let mut ld = LockDelay::new();
        ld.start();
        ld.reset();
        assert_eq!(ld.move_reset_count, 1);
        assert!(ld.active);
    }

    #[test]
    fn test_15_resets_still_running() {
        let mut ld = LockDelay::new();
        ld.start();
        for _ in 0..15 {
            ld.reset();
        }
        assert_eq!(ld.move_reset_count, 15);
        assert!(!ld.update());
        assert!(ld.active);
    }

    #[test]
    fn test_cancel_clears_state() {
        let mut ld = LockDelay::new();
        ld.start();
        ld.cancel();
        assert!(!ld.active);
        assert_eq!(ld.move_reset_count, 0);
        assert!(!ld.update());
        assert_eq!(ld.remaining_ms(), 0);
    }

    #[test]
    fn test_reset_activates_inactive() {
        let mut ld = LockDelay::new();
        assert!(!ld.active);
        ld.reset();
        assert!(ld.active);
        assert_eq!(ld.move_reset_count, 1);
        assert!(ld.remaining_ms() > 0);
    }
}

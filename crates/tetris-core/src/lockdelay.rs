use instant::Instant;
use std::time::Duration;

pub const MAX_MOVE_RESETS: u32 = 15;
pub const LOCK_DELAY_MS: u64 = 500;
pub const LOCK_DELAY_TICKS: u8 = 30;

/// Client-side wall-clock lock delay (non-deterministic, for UI only).
#[derive(Debug, Clone)]
pub struct LockDelay {
    active: bool,
    pub move_reset_count: u32,
    lock_deadline: Instant,
}

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

/// Server-side tick-based lock delay (deterministic, authoritative).
#[derive(Debug, Clone)]
pub struct LockDelayTicks {
    active: bool,
    pub move_reset_count: u8,
    accumulated_ticks: u8,
}

impl LockDelayTicks {
    pub fn new() -> Self {
        LockDelayTicks {
            active: false,
            move_reset_count: 0,
            accumulated_ticks: 0,
        }
    }

    pub fn start(&mut self) {
        if !self.active {
            self.active = true;
        }
    }

    pub fn reset(&mut self) {
        self.active = true;
        self.move_reset_count = self.move_reset_count.saturating_add(1);
        self.accumulated_ticks = 0;
    }

    pub fn update(&mut self) -> bool {
        if !self.active {
            return false;
        }

        if self.move_reset_count >= MAX_MOVE_RESETS as u8 {
            self.accumulated_ticks = self.accumulated_ticks.saturating_add(1);
            if self.accumulated_ticks >= LOCK_DELAY_TICKS {
                self.active = false;
                self.move_reset_count = 0;
                self.accumulated_ticks = 0;
                return true;
            }
            return false;
        }

        self.accumulated_ticks = self.accumulated_ticks.saturating_add(1);
        if self.accumulated_ticks >= LOCK_DELAY_TICKS {
            self.active = false;
            self.move_reset_count = 0;
            self.accumulated_ticks = 0;
            return true;
        }
        false
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.move_reset_count = 0;
        self.accumulated_ticks = 0;
    }

    pub fn remaining_ticks(&self) -> u8 {
        if !self.active {
            return 0;
        }
        LOCK_DELAY_TICKS.saturating_sub(self.accumulated_ticks)
    }
}

impl Default for LockDelayTicks {
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

    // ── LockDelayTicks tests ──

    #[test]
    fn test_ticks_not_active_by_default() {
        let mut ld = LockDelayTicks::new();
        assert!(!ld.active);
        assert_eq!(ld.move_reset_count, 0);
        assert!(!ld.update());
        assert_eq!(ld.remaining_ticks(), 0);
    }

    #[test]
    fn test_ticks_start_is_idempotent() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        ld.start();
        assert!(ld.active);
    }

    #[test]
    fn test_ticks_reset_increments_counter() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        ld.reset();
        assert_eq!(ld.move_reset_count, 1);
        assert!(ld.active);
    }

    #[test]
    fn test_ticks_lock_after_30_updates() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        for _ in 0..29 {
            assert!(!ld.update(), "should not lock before 30 ticks");
        }
        assert!(ld.update(), "should lock on 30th tick");
        assert!(!ld.active);
    }

    #[test]
    fn test_ticks_29_updates_not_locked() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        for _ in 0..29 {
            ld.update();
        }
        assert!(ld.active);
    }

    #[test]
    fn test_ticks_15_resets_still_running() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        for _ in 0..15 {
            ld.reset();
        }
        assert_eq!(ld.move_reset_count, 15);
        assert!(ld.active);
    }

    #[test]
    fn test_ticks_16th_reset_capped() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        for _ in 0..16 {
            ld.reset();
        }
        assert_eq!(ld.move_reset_count, 16);
    }

    #[test]
    fn test_ticks_locks_after_16_resets_and_30_ticks() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        for _ in 0..16 {
            ld.reset();
        }
        for _ in 0..29 {
            assert!(!ld.update());
        }
        assert!(ld.update());
    }

    #[test]
    fn test_ticks_cancel_clears_state() {
        let mut ld = LockDelayTicks::new();
        ld.start();
        ld.cancel();
        assert!(!ld.active);
        assert_eq!(ld.move_reset_count, 0);
        assert!(!ld.update());
        assert_eq!(ld.remaining_ticks(), 0);
    }

    #[test]
    fn test_ticks_reset_activates_inactive() {
        let mut ld = LockDelayTicks::new();
        assert!(!ld.active);
        ld.reset();
        assert!(ld.active);
        assert_eq!(ld.move_reset_count, 1);
        assert!(ld.remaining_ticks() > 0);
    }
}

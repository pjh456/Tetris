use serde::{Deserialize, Serialize};

use crate::attack::AttackResult;

const BASE_POINTS: [u32; 5] = [0, 100, 300, 500, 800];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreTracker {
    pub score: u32,
    pub level: u32,
    pub total_lines: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub b2b_count: u32,
    pub tspin_count: u32,
    pub all_clear_count: u32,
    pub game_time_ms: u64,
    pub total_pieces: u32,
    pub best_score: u32,
}

impl ScoreTracker {
    pub fn update(&mut self, attack: &AttackResult, lines_cleared: u8) {
        if lines_cleared > 0 {
            let idx = (lines_cleared as usize).min(4);
            let mut base = BASE_POINTS[idx] * self.level.max(1);
            if attack.is_tspin {
                base *= 2;
            }
            self.score += base;
            self.total_lines += lines_cleared as u32;
            self.level = self.total_lines / 10 + 1;
            self.combo += 1;
            self.max_combo = self.max_combo.max(self.combo);
        } else {
            self.combo = 0;
        }

        if attack.is_b2b {
            self.b2b_count += 1;
        }
        if attack.is_tspin {
            self.tspin_count += 1;
        }
        if attack.perfect_clear {
            self.all_clear_count += 1;
        }

        self.total_pieces += 1;
        self.best_score = self.best_score.max(self.score);
    }

    pub fn tick_time(&mut self, delta_ms: u64) {
        self.game_time_ms += delta_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_attack(is_tspin: bool, is_b2b: bool, perfect_clear: bool) -> AttackResult {
        AttackResult {
            damage: 0,
            is_tspin,
            is_mini: false,
            is_b2b,
            perfect_clear,
        }
    }

    #[test]
    fn test_score_1_line() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(false, false, false), 1);
        assert_eq!(s.score, 100);
    }

    #[test]
    fn test_score_4_lines() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(false, false, false), 4);
        assert_eq!(s.score, 800);
    }

    #[test]
    fn test_score_tspin_double() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(true, false, false), 2);
        assert_eq!(s.score, 600);
    }

    #[test]
    fn test_level_up_after_10_lines() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        for _ in 0..10 {
            s.update(&make_attack(false, false, false), 1);
        }
        assert_eq!(s.total_lines, 10);
        assert_eq!(s.level, 2);
    }

    #[test]
    fn test_b2b_counter() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(false, true, false), 4);
        assert_eq!(s.b2b_count, 1);
        s.update(&make_attack(false, true, false), 4);
        assert_eq!(s.b2b_count, 2);
    }

    #[test]
    fn test_combo_and_max_combo() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(false, false, false), 1);
        assert_eq!(s.combo, 1);
        s.update(&make_attack(false, false, false), 2);
        assert_eq!(s.combo, 2);
        assert_eq!(s.max_combo, 2);
        s.update(&AttackResult::default(), 0);
        assert_eq!(s.combo, 0);
        assert_eq!(s.max_combo, 2);
    }

    #[test]
    fn test_best_score_tracks_max() {
        let mut s = ScoreTracker::default();
        s.level = 1;
        s.update(&make_attack(false, false, false), 4);
        assert_eq!(s.best_score, 800);
        s.score = 0;
        s.update(&make_attack(false, false, false), 1);
        assert_eq!(s.best_score, 800);
    }
}

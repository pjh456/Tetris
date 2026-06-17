use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreTracker {
    pub score: u64,
    pub level: u32,
    pub total_lines: u32,
    pub combo: u32,
    pub max_combo: u32,
    pub b2b_count: u32,
    pub tspin_count: u32,
    pub all_clear_count: u32,
    pub game_time_ms: u64,
    pub total_pieces: u32,
    pub best_score: u64,
}

impl ScoreTracker {
    #[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
    pub fn update(
        &mut self,
        lines_cleared: u8,
        is_tspin: bool,
        is_mini: bool,
        is_b2b_clear: bool,
        perfect_clear: bool,
        soft_drop_cells: u8,
        hard_drop_cells: u8,
        combo_count: u32,
        level: u32,
    ) {
        let level = level.max(1);

        self.score += u64::from(soft_drop_cells);
        self.score += u64::from(hard_drop_cells) * 2;

        if lines_cleared > 0 {
            let base: u64 = if is_tspin {
                if is_mini {
                    match lines_cleared {
                        1 => 200,
                        _ => 200,
                    }
                } else {
                    match lines_cleared {
                        1 => 800,
                        2 => 1200,
                        3 => 1600,
                        _ => 1600,
                    }
                }
            } else {
                match lines_cleared {
                    1 => 100,
                    2 => 300,
                    3 => 500,
                    4 => 800,
                    _ => 800,
                }
            };

            let mut action_score = base * u64::from(level);

            if is_b2b_clear {
                action_score = (action_score * 3) / 2;
            }

            self.score += action_score;
            self.score += 50 * u64::from(combo_count) * u64::from(level);

            if perfect_clear {
                let pc_bonus: u64 = if is_b2b_clear && lines_cleared == 4 {
                    3200
                } else {
                    match lines_cleared {
                        1 => 800,
                        2 => 1200,
                        3 => 1800,
                        4 => 2000,
                        _ => 2000,
                    }
                };
                self.score += pc_bonus * u64::from(level);
            }

            self.total_lines += lines_cleared as u32;
            self.level = level.max(self.total_lines / 10 + 1).min(15);
            self.combo = combo_count.saturating_add(1);
            self.max_combo = self.max_combo.max(self.combo);
        } else {
            if is_tspin {
                let base: u64 = if is_mini { 100 } else { 400 };
                self.score += base * u64::from(level);
            }
            self.combo = 0;
        }

        if is_b2b_clear {
            self.b2b_count += 1;
        }
        if is_tspin {
            self.tspin_count += 1;
        }
        if perfect_clear {
            self.all_clear_count += 1;
        }

        self.total_pieces += 1;
        self.best_score = self.best_score.max(self.score);
    }

    pub fn tick_time(&mut self, delta_ms: u64) {
        self.game_time_ms += delta_ms;
    }
}

use tetris_core::Board;

/// Reward configuration. The dense signal is a per-placement **survival bonus**
/// that strictly dominates the small hole penalty, so longer episodes always
/// score higher — this structurally prevents the suicide failure mode that a
/// net-negative shaped reward induced. Line clears are the large positive
/// objective signal.
#[derive(Debug, Clone, Copy)]
pub struct RewardConfig {
    /// Per-placement survival bonus (dense, positive — the dominant signal).
    pub alive: f32,
    /// Penalty per newly created hole (small; only breaks ties toward clean play).
    pub hole_penalty: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            alive: 1.0,
            hole_penalty: 0.1,
        }
    }
}

/// Line-clear bonus. Strongly positive and tiered so the agent prefers bigger
/// clears; t-spin and perfect-clear outrank a plain tetris.
fn clear_bonus(lines_cleared: u8, is_tspin: bool, perfect_clear: bool) -> f32 {
    if perfect_clear {
        12.0
    } else if is_tspin && lines_cleared > 0 {
        8.0
    } else {
        match lines_cleared {
            4 => 8.0,
            3 => 5.0,
            2 => 3.0,
            1 => 1.0,
            _ => 0.0,
        }
    }
}

/// Survival-dominant reward. Per non-terminal step: `alive + clear − hole_penalty·Δholes`,
/// which is ≥ ~0.6 for realistic Δholes, so surviving is always rewarded. Game
/// over ends the alive stream and yields a small fixed penalty.
pub fn compute_reward(
    lines_cleared: u8,
    is_tspin: bool,
    perfect_clear: bool,
    game_over: bool,
    board_before: &Board<10, 20>,
    board_after: &Board<10, 20>,
    config: &RewardConfig,
) -> f32 {
    if game_over {
        return -1.0;
    }

    let new_holes = board_after.holes().saturating_sub(board_before.holes());
    let hole_term = config.hole_penalty * f32::from(u16::try_from(new_holes).unwrap_or(u16::MAX));

    config.alive + clear_bonus(lines_cleared, is_tspin, perfect_clear) - hole_term
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board_with_extra_hole() -> Board<10, 20> {
        // One covered hole: block at row 18, gap at row 19 col 0.
        let mut board = Board::<10, 20>::new();
        board.rows[18] = 1;
        board
    }

    #[test]
    fn survival_step_is_positive() {
        let board = Board::<10, 20>::new();
        let r = compute_reward(
            0,
            false,
            false,
            false,
            &board,
            &board,
            &RewardConfig::default(),
        );
        assert!((r - 1.0).abs() < 1e-6, "no-op survival step = alive bonus");
    }

    #[test]
    fn game_over_is_penalized() {
        let board = Board::<10, 20>::new();
        let r = compute_reward(
            0,
            false,
            false,
            true,
            &board,
            &board,
            &RewardConfig::default(),
        );
        assert!((r + 1.0).abs() < 1e-6);
        assert!(r < 1.0, "death must score below a survival step");
    }

    #[test]
    fn clears_dominate_survival() {
        let board = Board::<10, 20>::new();
        let cfg = RewardConfig::default();
        let tetris = compute_reward(4, false, false, false, &board, &board, &cfg);
        let single = compute_reward(1, false, false, false, &board, &board, &cfg);
        let survive = compute_reward(0, false, false, false, &board, &board, &cfg);
        assert!(tetris > single && single > survive);
        assert!((tetris - (1.0 + 8.0)).abs() < 1e-6);
    }

    #[test]
    fn perfect_clear_outranks_tetris() {
        let board = Board::<10, 20>::new();
        let cfg = RewardConfig::default();
        let pc = compute_reward(4, false, true, false, &board, &board, &cfg);
        let tetris = compute_reward(4, false, false, false, &board, &board, &cfg);
        assert!(pc > tetris);
    }

    #[test]
    fn new_holes_are_penalized_but_step_stays_positive() {
        let cfg = RewardConfig::default();
        let before = Board::<10, 20>::new();
        let after = board_with_extra_hole();
        let r = compute_reward(0, false, false, false, &before, &after, &cfg);
        // alive 1.0 − 0.1·(1 new hole) = 0.9, still positive (survival rewarded).
        assert!((r - 0.9).abs() < 1e-6);
        assert!(r > 0.0);
    }
}

use tetris_core::Board;

/// Negative-weighted board-quality features. Each weight is `<= 0`, so a messier
/// board (more holes/height/bumpiness) yields a more negative potential. The
/// empty board scores `0` (all features are `0`), which keeps the potential a
/// proper Ng-1999 shaping function (telescopes to zero on a clean board).
#[derive(Debug, Clone, Copy)]
pub struct PotentialWeights {
    pub holes: f32,
    pub height: f32,
    pub bump: f32,
    pub wells: f32,
    pub row_trans: f32,
    pub covered: f32,
    pub max_h: f32,
}

impl PotentialWeights {
    /// All-zero weights: reduces the reward to the pure prior-art `lines^2 * W`
    /// recipe (shaping term vanishes) — the documented fallback if shaping
    /// misbehaves. No code change needed, just construct with this.
    pub const ZERO: Self = Self {
        holes: 0.0,
        height: 0.0,
        bump: 0.0,
        wells: 0.0,
        row_trans: 0.0,
        covered: 0.0,
        max_h: 0.0,
    };
}

impl Default for PotentialWeights {
    fn default() -> Self {
        // Small negatives: shaping is a SECONDARY density signal, kept well below
        // the dominant clear bonus (single = 10). Per-placement potential deltas
        // stay O(1). Tune here (or use ZERO) without touching the reward logic.
        Self {
            holes: -0.5,
            height: -0.01,
            bump: -0.05,
            wells: -0.1,
            row_trans: -0.03,
            covered: -0.05,
            max_h: -0.1,
        }
    }
}

/// Reward configuration. The dominant signal is a **superlinear line-clear bonus**
/// (`lines^2 * clear_width`); potential-based shaping adds density without
/// changing the optimal policy (Ng 1999); the game-over step carries ONLY the
/// death penalty (no shaping, no clear) so the terminal `-phi(s)` leak that
/// induced suicide in 08-07 cannot pay out.
#[derive(Debug, Clone, Copy)]
pub struct RewardConfig {
    pub potential: PotentialWeights,
    /// Shaping discount; MUST match the training gamma (Ng 1999, Pitfall 7).
    pub gamma: f32,
    /// SMALL per-placement survival bonus. Pure `lines^2*W` is too sparse — with
    /// zero reward for every non-clearing step the landscape is flat and PPO has
    /// no gradient (it plateaued at `ep_rew` ≈ -death, `ep_len` ~27, never discovering
    /// clears). A small alive bonus gives a climbing gradient toward longer
    /// episodes; since the board tops out in ~50 placements WITHOUT clearing,
    /// "survive longer" forces "clear lines". Kept small so a clear (10..160) still
    /// dominates an episode's survival sum — avoids the 08-07-flat pack-and-coast
    /// local optimum where a large alive bonus made minimal clearing "good enough".
    pub alive: f32,
    /// Fixed penalty on the game-over step; large enough that dying is never profitable.
    pub death_penalty: f32,
    /// Line-clear multiplier (board width). `clear = lines^2 * clear_width`.
    pub clear_width: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            // ZERO shaping = pure prior-art `lines^2 * W` + death (nuno-faria/uvipen
            // recipe). The negative-weighted PotentialWeights::default() punishes
            // every surviving step (Phi drops as the stack grows even under good
            // play), which created a "die early to stop accruing negative shaping"
            // gradient BEFORE the agent discovered clearing — a second suicide
            // relapse (ep_len fell 19 -> 12 at 130k steps). Pure clear-dominant
            // reward has no survival-punishing term: dying is just -50, surviving
            // does nothing, clearing is the only way up. Re-enable shaping later
            // only with non-height features if density is needed.
            potential: PotentialWeights::ZERO,
            gamma: 0.99,
            alive: 0.1,
            death_penalty: 50.0,
            clear_width: 10.0,
        }
    }
}

/// Board-quality potential. Weighted sum of board getters; `0` on the empty board,
/// strictly `< 0` for any board with holes/height (given the negative Default weights).
pub fn potential(board: &Board<10, 20>, w: &PotentialWeights) -> f32 {
    // Board features are bounded by the 10x20 grid (<= 400), so the lossless
    // u32 -> u16 -> f32 path never truncates or loses precision.
    let feat = |x: u32| f32::from(u16::try_from(x).unwrap_or(u16::MAX));
    let max_h = f32::from(board.column_heights().iter().copied().max().unwrap_or(0));
    w.holes * feat(board.holes())
        + w.height * feat(board.aggregate_height())
        + w.bump * feat(board.bumpiness())
        + w.wells * feat(board.wells())
        + w.row_trans * feat(board.row_transitions())
        + w.covered * feat(board.covered_holes())
        + w.max_h * max_h
}

/// Superlinear clear bonus: `lines^2 * width`, so a tetris (160 at W=10) far
/// outscores four singles (40). T-spins and perfect clears get a multiplier so
/// they outrank a plain clear of the same line count. Zero lines = zero bonus.
fn clear_bonus(lines_cleared: u8, is_tspin: bool, perfect_clear: bool, width: f32) -> f32 {
    if lines_cleared == 0 {
        return 0.0;
    }
    let base = (f32::from(lines_cleared)).powi(2) * width;
    if perfect_clear {
        base * 2.0
    } else if is_tspin {
        base * 1.5
    } else {
        base
    }
}

/// Clear-dominant reward with terminal-guarded potential shaping.
///
/// - **game over**: returns exactly `-death_penalty` — no shaping, no clear. This
///   closes the 08-07 suicide leak (a `gamma*phi(terminal) - phi(s)` term with
///   `phi(s) < 0` paid the agent for dying on a messy board).
/// - **otherwise**: `(gamma*phi(after) - phi(before)) + clear_bonus`.
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
        return -config.death_penalty;
    }

    let shaping = config.gamma * potential(board_after, &config.potential)
        - potential(board_before, &config.potential);
    config.alive + shaping + clear_bonus(lines_cleared, is_tspin, perfect_clear, config.clear_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Board<10, 20> {
        Board::<10, 20>::new()
    }

    /// One covered hole: block at row 18, gap at row 19 — holes() > 0, so the
    /// default-weighted potential is strictly negative.
    fn messy() -> Board<10, 20> {
        let mut board = empty();
        board.rows[18] = 1;
        board
    }

    #[test]
    fn clear_bonus_is_superlinear() {
        // Isolate the clear term: zero alive + zero shaping => reward == clear_bonus.
        let cfg = RewardConfig {
            alive: 0.0,
            ..RewardConfig::default()
        };
        let e = empty();
        // Same before/after empty board => shaping == 0 => reward == clear_bonus.
        let single = compute_reward(1, false, false, false, &e, &e, &cfg);
        let double = compute_reward(2, false, false, false, &e, &e, &cfg);
        let tetris = compute_reward(4, false, false, false, &e, &e, &cfg);
        assert!((single - 10.0).abs() < 1e-6);
        assert!((double - 40.0).abs() < 1e-6);
        assert!((tetris - 160.0).abs() < 1e-6);
        assert!(tetris > 4.0 * single, "a tetris must beat four singles");
        assert!(double > 2.0 * single, "a double must beat two singles");
    }

    #[test]
    fn tspin_and_perfect_clear_outrank_plain() {
        let cfg = RewardConfig::default();
        let e = empty();
        let plain_single = compute_reward(1, false, false, false, &e, &e, &cfg);
        let tspin_single = compute_reward(1, true, false, false, &e, &e, &cfg);
        let perfect = compute_reward(2, false, true, false, &e, &e, &cfg);
        let plain_double = compute_reward(2, false, false, false, &e, &e, &cfg);
        assert!(tspin_single > plain_single);
        assert!(perfect > plain_double);
    }

    #[test]
    fn game_over_is_only_death_penalty_regardless_of_board() {
        let cfg = RewardConfig::default();
        // Even a messy board with a "clear" flagged: game over pays exactly -death.
        let r = compute_reward(4, true, true, true, &messy(), &messy(), &cfg);
        assert!(
            (r + 50.0).abs() < 1e-6,
            "game over == -death_penalty, no leak"
        );
    }

    #[test]
    fn dying_a_messy_board_never_pays() {
        // The exact 08-07 regression: a -phi(messy) terminal bonus must NOT exist.
        let cfg = RewardConfig::default();
        let over = compute_reward(0, false, false, true, &messy(), &messy(), &cfg);
        assert!(over < 0.0, "dying must be negative");
        assert!((over + 50.0).abs() < 1e-6);
    }

    #[test]
    fn any_nonterminal_step_beats_game_over() {
        let cfg = RewardConfig::default();
        let over = compute_reward(0, false, false, true, &messy(), &messy(), &cfg);
        // Worst realistic live step: empty -> messy, no clear (shaping goes negative).
        let live_bad = compute_reward(0, false, false, false, &empty(), &messy(), &cfg);
        assert!(live_bad > over, "even a bad non-terminal step beats dying");
    }

    #[test]
    fn potential_zero_on_empty_negative_on_messy() {
        let w = PotentialWeights::default();
        assert!(
            (potential(&empty(), &w)).abs() < 1e-6,
            "empty board potential == 0"
        );
        assert!(potential(&messy(), &w) < 0.0, "messy board potential < 0");
    }

    #[test]
    fn zeroed_weights_reduce_to_pure_clear_bonus() {
        let cfg = RewardConfig {
            potential: PotentialWeights::ZERO,
            alive: 0.0,
            ..RewardConfig::default()
        };
        // Shaping vanishes (phi == 0 everywhere) => live reward == clear bonus exactly,
        // even on a messy board with before != after.
        let r = compute_reward(1, false, false, false, &empty(), &messy(), &cfg);
        assert!(
            (r - 10.0).abs() < 1e-6,
            "zeroed shaping + alive => pure lines^2*W"
        );
    }

    #[test]
    fn survival_step_pays_small_alive() {
        // Default: ZERO shaping, no clear, not over => reward == alive (the small
        // dense gradient that breaks the sparse -death plateau). Must be << a single
        // clear (10) so clearing still dominates an episode.
        let cfg = RewardConfig::default();
        let r = compute_reward(0, false, false, false, &empty(), &empty(), &cfg);
        assert!(
            (r - 0.1).abs() < 1e-6,
            "no-clear survival step pays the alive bonus"
        );
        assert!(
            r > 0.0 && r < 10.0,
            "alive is positive but far below a single clear"
        );
    }
}

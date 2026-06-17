use tetris_core::Board;

#[derive(Debug, Clone, Copy)]
pub struct RewardConfig {
    pub hole_penalty: f32,
    pub height_penalty: f32,
    pub bumpiness_penalty: f32,
    pub well_penalty: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            hole_penalty: -0.5,
            height_penalty: -0.3,
            bumpiness_penalty: -0.2,
            well_penalty: -0.3,
        }
    }
}

pub fn compute_reward(
    lines_cleared: u8,
    game_over: bool,
    board_before: &Board<10, 20>,
    board_after: &Board<10, 20>,
    config: RewardConfig,
) -> f32 {
    if game_over {
        return -1.0;
    }

    let baseline = if lines_cleared == 4 {
        4.0
    } else {
        f32::from(lines_cleared)
    };

    baseline
        + feature_delta(
            board_before.holes(),
            board_after.holes(),
            config.hole_penalty,
        )
        + feature_delta(
            board_before.aggregate_height(),
            board_after.aggregate_height(),
            config.height_penalty,
        )
        + feature_delta(
            board_before.bumpiness(),
            board_after.bumpiness(),
            config.bumpiness_penalty,
        )
        + feature_delta(
            board_before.wells(),
            board_after.wells(),
            config.well_penalty,
        )
}

fn feature_delta(before: u32, after: u32, weight: f32) -> f32 {
    let before = u16::try_from(before).unwrap_or(u16::MAX);
    let after = u16::try_from(after).unwrap_or(u16::MAX);
    (f32::from(after) - f32::from(before)) * weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_reward_returns_zero_for_no_change() {
        let board = Board::<10, 20>::new();
        assert_eq!(
            compute_reward(0, false, &board, &board, RewardConfig::default()),
            0.0
        );
    }

    #[test]
    fn compute_reward_returns_negative_one_for_game_over() {
        let board = Board::<10, 20>::new();
        assert_eq!(
            compute_reward(4, true, &board, &board, RewardConfig::default()),
            -1.0
        );
    }

    #[test]
    fn compute_reward_uses_tetris_baseline() {
        let board = Board::<10, 20>::new();
        assert_eq!(
            compute_reward(4, false, &board, &board, RewardConfig::default()),
            4.0
        );
    }
}

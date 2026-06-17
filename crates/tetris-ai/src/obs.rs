use tetris_core::{Rot, State};

pub const BOARD_W: usize = 10;
pub const BOARD_H: usize = 20;
pub const PIECE_COUNT: usize = 7;
pub const ACTION_SPACE_SIZE: usize = BOARD_W * 4;
pub const OBS_DIM: usize = BOARD_W * BOARD_H + PIECE_COUNT + PIECE_COUNT * 5 + 2;

pub fn encode_obs(state: &State<BOARD_W, BOARD_H>) -> Vec<f32> {
    let mut obs = Vec::with_capacity(OBS_DIM);

    for row in 0..BOARD_H {
        for col in 0..BOARD_W {
            obs.push(if state.board.rows[row] & (1u64 << col) != 0 {
                1.0
            } else {
                0.0
            });
        }
    }

    push_piece_one_hot(&mut obs, state.hold as usize);
    for piece in state.next {
        push_piece_one_hot(&mut obs, piece as usize);
    }

    let combo = u8::try_from(state.combo.clamp(0, 20)).unwrap_or(0);
    obs.push(f32::from(combo) / 20.0);
    obs.push(f32::from(state.b2b));
    debug_assert_eq!(obs.len(), OBS_DIM);
    obs
}

pub fn action_mask(placements: &[(i8, Rot)]) -> Vec<bool> {
    let mut mask = vec![false; ACTION_SPACE_SIZE];
    for &(col, rot) in placements {
        if let Some(index) = placement_to_action(col, rot) {
            mask[index] = true;
        }
    }
    mask
}

pub fn action_to_placement(action: usize) -> Option<(i8, Rot)> {
    if action >= ACTION_SPACE_SIZE {
        return None;
    }
    let col = (action / 4) as i8;
    let rot = match action % 4 {
        0 => Rot::R0,
        1 => Rot::R90,
        2 => Rot::R180,
        3 => Rot::R270,
        _ => return None,
    };
    Some((col, rot))
}

pub fn placement_to_action(col: i8, rot: Rot) -> Option<usize> {
    if !(0..BOARD_W as i8).contains(&col) {
        return None;
    }
    Some(col as usize * 4 + rot as usize)
}

fn push_piece_one_hot(obs: &mut Vec<f32>, piece_index: usize) {
    for idx in 0..PIECE_COUNT {
        obs.push(if idx == piece_index { 1.0 } else { 0.0 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::Engine;

    #[test]
    fn encode_obs_returns_expected_dimension() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        assert_eq!(encode_obs(&engine.state).len(), OBS_DIM);
    }

    #[test]
    fn action_mask_returns_fixed_action_space_size() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let mask = action_mask(&engine.enumerate_placements());
        assert_eq!(mask.len(), ACTION_SPACE_SIZE);
    }

    #[test]
    fn action_to_placement_maps_flat_index() {
        assert_eq!(action_to_placement(5), Some((1, Rot::R90)));
    }
}

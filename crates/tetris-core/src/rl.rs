use crate::{Engine, Rot, State};

pub const BOARD_W: usize = 10;
pub const BOARD_H: usize = 20;
pub const PIECE_COUNT: usize = 7;
pub const NEXT_COUNT: usize = 5;
pub const ACTION_SPACE_SIZE: usize = BOARD_W * 4;
pub const OBS_DIM: usize = 10
    + 1
    + 1
    + 1
    + 1
    + 1
    + 1
    + 1
    + PIECE_COUNT
    + PIECE_COUNT * NEXT_COUNT
    + 1
    + 1
    + BOARD_W
    + (BOARD_W - 1);

const BOARD_HEIGHT_DIVISOR: f32 = 20.0;
const BOARD_AREA_DIVISOR: f32 = 200.0;
const WELLS_DIVISOR: f32 = 40.0;
const COMBO_DIVISOR: f32 = 20.0;

pub fn encode_obs(state: &State<BOARD_W, BOARD_H>) -> Vec<f32> {
    let mut obs = Vec::with_capacity(OBS_DIM);
    encode_obs_into(&mut obs, state);
    obs
}

pub fn encode_obs_into(obs: &mut Vec<f32>, state: &State<BOARD_W, BOARD_H>) {
    let heights = state.board.column_heights();

    for height in heights {
        obs.push(f32::from(height) / BOARD_HEIGHT_DIVISOR);
    }

    obs.push(u32_to_unit(state.board.holes(), BOARD_AREA_DIVISOR));
    obs.push(u32_to_unit(
        state.board.aggregate_height(),
        BOARD_AREA_DIVISOR,
    ));
    obs.push(u32_to_unit(state.board.bumpiness(), BOARD_AREA_DIVISOR));
    obs.push(u32_to_unit(state.board.wells(), WELLS_DIVISOR));
    obs.push(f32::from(heights.into_iter().max().unwrap_or(0)) / BOARD_HEIGHT_DIVISOR);
    obs.push(u32_to_unit(
        state.board.row_transitions(),
        BOARD_AREA_DIVISOR,
    ));
    obs.push(u32_to_unit(state.board.covered_holes(), BOARD_AREA_DIVISOR));

    // column_overhang: depth of first hole below column top, /20.
    // Exposes T-Slot overhang structure to the value MLP.
    for col in 0..BOARD_W {
        let h = heights[col];
        if h == 0 {
            obs.push(0.0);
        } else {
            let start_row = BOARD_H as u8 - h;
            let mut overhang = 0u8;
            for r in start_row..BOARD_H as u8 {
                if state.board.rows[r as usize] & (1u64 << col) != 0 {
                    break;
                }
                overhang += 1;
            }
            obs.push(f32::from(overhang) / BOARD_HEIGHT_DIVISOR);
        }
    }

    // adjacent_height_diff: abs difference between adjacent column heights, /20.
    for c in 0..BOARD_W - 1 {
        let diff = (heights[c] as i16 - heights[c + 1] as i16).unsigned_abs();
        obs.push(f32::from(diff as u8) / BOARD_HEIGHT_DIVISOR);
    }

    push_piece_one_hot(obs, state.hold as usize);
    for piece in state.next {
        push_piece_one_hot(obs, piece as usize);
    }

    let combo = u8::try_from(state.combo.clamp(0, 20)).unwrap_or(0);
    obs.push(f32::from(combo) / COMBO_DIVISOR);
    obs.push(u8::from(state.b2b).into());
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
    let col = i8::try_from(action / 4).ok()?;
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
    // Negative `col` denotes an off-board (left-overhang) placement, which is not
    // a legal landing column — `try_from` rejects it (returns None) so it is
    // excluded from the action space. Intentionally NOT extended: the action
    // space [0, BOARD_W*4) matches the trained policy weights; widening it would
    // invalidate existing models. Such placements are extreme/degenerate and the
    // policy does not rely on them.
    let col = usize::try_from(col).ok()?;
    if col >= BOARD_W {
        return None;
    }
    Some(col * 4 + rot as usize)
}

/// Contiguous flat-buffer variant of `afterstate_features` for zero-copy FFI.
/// Returns `(actions [n], features [n * OBS_DIM])` built with a single allocation.
/// The inference path (`afterstate_features`) is unchanged (CONTEXT D9 freeze).
pub fn afterstate_features_flat(engine: &Engine<BOARD_W, BOARD_H>) -> (Vec<i64>, Vec<f32>) {
    let placements = engine.enumerate_placements();
    let n = placements.len();
    let mut actions = Vec::with_capacity(n);
    let mut flat = Vec::with_capacity(n * OBS_DIM);
    for (col, rot) in placements {
        let Some(action) = placement_to_action(col, rot) else {
            continue;
        };
        let Some(state) = engine.ghost_afterstate(col, rot) else {
            continue;
        };
        actions.push(i64::try_from(action).unwrap_or(0));
        let before = flat.len();
        encode_obs_into(&mut flat, &state);
        debug_assert_eq!(flat.len() - before, OBS_DIM);
    }
    debug_assert_eq!(actions.len() * OBS_DIM, flat.len());
    (actions, flat)
}

/// For every legal placement, the engineered obs of the board that WOULD result
/// from that placement — computed on a clone, leaving `engine` unmutated. Returns
/// `(action, features)` pairs where `action` is the same index as
/// `action_to_placement` (range `0..ACTION_SPACE_SIZE`). Placements with no valid
/// action index (off-board overhang, see `placement_to_action`) are skipped,
/// matching `action_mask`. Substrate for afterstate value learning/inference.
pub fn afterstate_features(engine: &Engine<BOARD_W, BOARD_H>) -> Vec<(usize, Vec<f32>)> {
    let mut out = Vec::new();
    for (col, rot) in engine.enumerate_placements() {
        let Some(action) = placement_to_action(col, rot) else {
            continue;
        };
        let mut clone = engine.clone();
        clone.apply_placement(col, rot);
        out.push((action, encode_obs(&clone.state)));
    }
    out
}

fn push_piece_one_hot(obs: &mut Vec<f32>, piece_index: usize) {
    for idx in 0..PIECE_COUNT {
        obs.push(if idx == piece_index { 1.0 } else { 0.0 });
    }
}

fn u32_to_unit(value: u32, divisor: f32) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value) / divisor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;

    #[test]
    fn encode_obs_returns_engineered_dimension() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        assert_eq!(encode_obs(&engine.state).len(), OBS_DIM);
        assert_eq!(OBS_DIM, 80);
    }

    #[test]
    fn encode_obs_values_are_finite_and_normalized() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let obs = encode_obs(&engine.state);
        assert!(obs.iter().all(|value| value.is_finite()));
        assert!(obs.iter().all(|value| (0.0..=1.0).contains(value)));
    }

    #[test]
    fn column_overhang_empty_board_all_zero() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        // Fresh board: no cells placed, all column heights are 0, so overhangs are 0.
        let obs = encode_obs(&engine.state);
        // column_overhang starts at index 17 (10 heights + 7 scalars = 17)
        for i in 17..27 {
            assert_eq!(obs[i], 0.0, "empty board overhang[{i}] must be 0");
        }
    }

    #[test]
    fn adjacent_height_diff_empty_board_all_zero() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let obs = encode_obs(&engine.state);
        // adjacent_height_diff starts at index 27 (17 + 10 = 27), 9 values
        for i in 27..36 {
            assert_eq!(obs[i], 0.0, "empty board adj_diff[{i}] must be 0");
        }
    }

    #[test]
    fn column_heights_occupy_ten_distinct_positions() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        engine.state.board.rows[19] = 0b11;
        let obs = encode_obs(&engine.state);
        assert_eq!(obs[0], 1.0 / 20.0);
        assert_eq!(obs[1], 1.0 / 20.0);
        assert_eq!(obs[2], 0.0);
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

    #[test]
    fn afterstate_features_does_not_mutate_engine() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let before = engine.state_hash();
        let feats = afterstate_features(&engine);
        assert!(!feats.is_empty(), "a fresh game has legal placements");
        assert_eq!(
            engine.state_hash(),
            before,
            "peek must not mutate the live engine"
        );
        assert!(
            feats.iter().all(|(_, f)| f.len() == OBS_DIM),
            "every afterstate vector is OBS_DIM"
        );
    }

    #[test]
    fn afterstate_features_match_applied_placement() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let feats = afterstate_features(&engine);
        assert!(!feats.is_empty());
        let action = feats[0].0;
        let peeked = feats[0].1.clone();
        if let Some((col, rot)) = action_to_placement(action) {
            let mut applied = engine.clone();
            applied.apply_placement(col, rot);
            assert_eq!(
                peeked,
                encode_obs(&applied.state),
                "peeked features == features of the applied clone"
            );
        } else {
            assert!(false, "first afterstate action must map to a placement");
        }
    }

    #[test]
    fn afterstate_features_flat_parity_with_pair_form() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let (actions_flat, flat) = afterstate_features_flat(&engine);
        let pairs = afterstate_features(&engine);
        assert_eq!(
            actions_flat.len(),
            pairs.len(),
            "flat and pair form have same action count"
        );
        assert_eq!(
            flat.len(),
            actions_flat.len() * OBS_DIM,
            "flat len == n_actions * OBS_DIM"
        );
        for (i, (action, f)) in pairs.iter().enumerate() {
            assert_eq!(
                usize::try_from(actions_flat[i]).unwrap(),
                *action,
                "action {i} matches"
            );
            let row = &flat[i * OBS_DIM..(i + 1) * OBS_DIM];
            assert_eq!(row, f.as_slice(), "flat row {i} == pair features[{i}]");
        }
    }

    #[test]
    fn afterstate_features_flat_does_not_mutate_engine() {
        let mut engine = Engine::<10, 20>::new();
        engine.reset(42);
        let before = engine.state_hash();
        let (actions, flat) = afterstate_features_flat(&engine);
        assert!(!actions.is_empty());
        assert_eq!(engine.state_hash(), before);
        assert_eq!(flat.len(), actions.len() * OBS_DIM);
    }
}

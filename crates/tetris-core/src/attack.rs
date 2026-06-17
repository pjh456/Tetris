use serde::{Deserialize, Serialize};

use crate::board::Board;
use crate::state::State;
use crate::types::Piece;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttackResult {
    pub damage: i32,
    pub hole_x: u8,
    pub is_tspin: bool,
    pub is_mini: bool,
    pub is_b2b: bool,
    pub perfect_clear: bool,
    pub garbage_inserted: bool,
}

pub const TSPIN_DMG: [i32; 5] = [0, 2, 4, 6, 8];
pub const NORMAL_DMG: [i32; 5] = [0, 0, 1, 2, 4];
pub const COMBO_DMG: [i32; 12] = [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4, 5];
pub const PC_BONUS: i32 = 10;

pub fn is_occupied<const W: usize, const H: usize>(board: &Board<W, H>, x: i32, y: i32) -> bool {
    if x < 0 || x >= W as i32 || y >= H as i32 || y < 0 {
        return true;
    }
    (board.rows[y as usize] & (1u64 << x)) != 0
}

pub fn check_t_spin<const W: usize, const H: usize>(st: &State<W, H>) -> bool {
    if st.piece != Piece::T || !st.last_move_was_rotation {
        return false;
    }
    corner_count(st) >= 2
}

fn check_full_t_spin<const W: usize, const H: usize>(st: &State<W, H>) -> bool {
    corner_count(st) >= 3
}

fn corner_count<const W: usize, const H: usize>(st: &State<W, H>) -> usize {
    let cx = st.x as i32 + 1;
    let cy = st.y as i32 + 1;

    let mut corners = 0;
    if is_occupied(&st.board, cx - 1, cy - 1) {
        corners += 1;
    }
    if is_occupied(&st.board, cx + 1, cy - 1) {
        corners += 1;
    }
    if is_occupied(&st.board, cx - 1, cy + 1) {
        corners += 1;
    }
    if is_occupied(&st.board, cx + 1, cy + 1) {
        corners += 1;
    }
    corners
}

pub fn calculate_attack<const W: usize, const H: usize>(
    st: &mut State<W, H>,
    lines_cleared: i32,
) -> AttackResult {
    let mut res = AttackResult::default();
    if lines_cleared == 0 {
        st.combo = 0;
        return res;
    }

    let lc = lines_cleared as usize;
    res.is_tspin = check_t_spin(st);
    res.is_mini = res.is_tspin && !check_full_t_spin(st);

    if res.is_tspin {
        res.damage = TSPIN_DMG[lc.min(4)];
    } else {
        res.damage = NORMAL_DMG[lc.min(4)];
    }

    let is_difficult_clear = lines_cleared == 4 || res.is_tspin;
    if is_difficult_clear {
        if st.b2b {
            res.damage += 1;
            res.is_b2b = true;
        }
        st.b2b = true;
    } else {
        st.b2b = false;
    }

    let max_combo_idx = COMBO_DMG.len() - 1;
    let combo_val = st.combo.max(0) as usize;
    let combo_idx = if combo_val < max_combo_idx {
        combo_val
    } else {
        max_combo_idx
    };
    res.damage += COMBO_DMG[combo_idx];
    st.combo += 1;

    res.perfect_clear = st.board.is_empty();
    if res.perfect_clear {
        res.damage += PC_BONUS;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::state::State;
    use crate::types::{Piece, Rot};

    fn base_state() -> State<10, 20> {
        State {
            board: Board::new(),
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 5],
            rng: 0,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            last_move_was_rotation: false,
            last_clear_mask: 0,
            last_clear_count: 0,
            last_harddrop_cols: 0,
            last_harddrop_start_y: 0,
            last_harddrop_end_y: 0,
            last_harddrop_piece: Piece::I,
            last_harddrop_valid: false,
        }
    }

    #[test]
    fn test_normal_damage_0() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 0);
        assert_eq!(res.damage, 0);
        assert_eq!(st.combo, 0);
    }

    #[test]
    fn test_normal_damage_1() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 1);
        assert_eq!(res.damage, 0);
    }

    #[test]
    fn test_normal_damage_2() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 2);
        assert_eq!(res.damage, 1);
    }

    #[test]
    fn test_normal_damage_3() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 3);
        assert_eq!(res.damage, 2);
    }

    #[test]
    fn test_normal_damage_4_tetris() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 4);
        assert_eq!(res.damage, 4);
        assert!(!res.is_b2b);
        assert!(st.b2b);
    }

    fn make_tspin_state() -> State<10, 20> {
        let mut st = State {
            board: Board::new(),
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 5,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 5],
            rng: 0,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            last_move_was_rotation: true,
            last_clear_mask: 0,
            last_clear_count: 0,
            last_harddrop_cols: 0,
            last_harddrop_start_y: 0,
            last_harddrop_end_y: 0,
            last_harddrop_piece: Piece::I,
            last_harddrop_valid: false,
        };
        let cx = st.x as i32 + 1;
        let cy = st.y as i32 + 1;
        st.board.rows[(cy - 1) as usize] |= 1u64 << (cx - 1); // top-left
        st.board.rows[(cy - 1) as usize] |= 1u64 << (cx + 1); // top-right
        st.board.rows[(cy + 1) as usize] |= 1u64 << (cx + 1); // bottom-right
        st
    }

    #[test]
    fn test_tspin_single() {
        let mut st = make_tspin_state();
        let res = calculate_attack(&mut st, 1);
        assert!(res.is_tspin);
        assert_eq!(res.damage, 2);
    }

    #[test]
    fn test_tspin_double() {
        let mut st = make_tspin_state();
        let res = calculate_attack(&mut st, 2);
        assert!(res.is_tspin);
        assert_eq!(res.damage, 4);
    }

    #[test]
    fn test_tspin_triple() {
        let mut st = make_tspin_state();
        let res = calculate_attack(&mut st, 3);
        assert!(res.is_tspin);
        assert_eq!(res.damage, 6);
    }

    #[test]
    fn test_not_tspin_without_rotation() {
        let mut st = make_tspin_state();
        st.last_move_was_rotation = false;
        let res = calculate_attack(&mut st, 2);
        assert!(!res.is_tspin);
    }

    #[test]
    fn test_b2b_first_tetris() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        let res = calculate_attack(&mut st, 4);
        assert!(!res.is_b2b);
        assert!(st.b2b);
        assert_eq!(res.damage, 4);
    }

    #[test]
    fn test_b2b_consecutive() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.b2b = true;
        let res = calculate_attack(&mut st, 4);
        assert!(res.is_b2b);
        assert!(st.b2b);
        assert_eq!(res.damage, 5);
    }

    #[test]
    fn test_b2b_break() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.b2b = true;
        let res = calculate_attack(&mut st, 2);
        assert!(!res.is_b2b);
        assert!(!st.b2b);
        assert_eq!(res.damage, 1);
    }

    #[test]
    fn test_b2b_tspin() {
        let mut st = make_tspin_state();
        let res = calculate_attack(&mut st, 2);
        assert!(res.is_tspin);
        assert!(st.b2b);
    }

    #[test]
    fn test_combo_0() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.combo = 0;
        let res = calculate_attack(&mut st, 1);
        assert_eq!(res.damage, 0);
        assert_eq!(st.combo, 1);
    }

    #[test]
    fn test_combo_1() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.combo = 1;
        let res = calculate_attack(&mut st, 1);
        assert_eq!(res.damage, 0);
        assert_eq!(st.combo, 2);
    }

    #[test]
    fn test_combo_2() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.combo = 2;
        let res = calculate_attack(&mut st, 1);
        assert_eq!(res.damage, 1);
        assert_eq!(st.combo, 3);
    }

    #[test]
    fn test_combo_3() {
        let mut st = base_state();
        st.board.rows[0] = 1;
        st.combo = 3;
        let res = calculate_attack(&mut st, 1);
        assert_eq!(res.damage, 1);
        assert_eq!(st.combo, 4);
    }

    #[test]
    fn test_perfect_clear() {
        let mut st = State::<10, 20> {
            board: Board::new(),
            piece: Piece::T,
            rot: Rot::R0,
            x: 3,
            y: 0,
            hold: Piece::I,
            hold_used: false,
            next: [Piece::I; 5],
            rng: 0,
            combo: 0,
            b2b: false,
            pending_garbage: 0,
            last_move_was_rotation: false,
            last_clear_mask: 0,
            last_clear_count: 0,
            last_harddrop_cols: 0,
            last_harddrop_start_y: 0,
            last_harddrop_end_y: 0,
            last_harddrop_piece: Piece::I,
            last_harddrop_valid: false,
        };
        let res = calculate_attack(&mut st, 4);
        assert!(res.perfect_clear);
        assert_eq!(res.damage, 14);
    }

    #[test]
    fn test_not_perfect_clear() {
        let mut st = base_state();
        st.board.rows[19] = 0x001;
        let res = calculate_attack(&mut st, 1);
        assert!(!res.perfect_clear);
        assert_eq!(res.damage, 0);
    }
}

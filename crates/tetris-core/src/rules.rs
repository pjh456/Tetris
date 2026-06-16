use crate::board::Board;
use crate::piece::PIECES;
use crate::srs;
use crate::state::State;
use crate::types::Rot;

pub fn can_place<const W: usize, const H: usize>(st: &State<W, H>, x: i8, y: i8, rot: Rot) -> bool {
    let shape = &PIECES[st.piece as usize].rot[rot as usize];

    let mut wall_mask: u16 = 0;
    let signed_shift = if x == i8::MIN { 15u16 } else { (-x).min(15) as u16 };
    if x < 0 {
        wall_mask |= (1u16 << signed_shift) - 1;
    }
    let w = W as i8;
    if x + 4 > w {
        let shift = (if w - x > 0 { w - x } else { 0 }).min(15);
        wall_mask |= 0xFFFFu16 << (shift as u16);
    }

    for i in 0..4 {
        let row = shape.row[i];
        if row == 0 {
            continue;
        }

        if row & wall_mask != 0 {
            return false;
        }

        let yy = y + i as i8;
        if yy >= H as i8 {
            return false;
        }
        if yy < 0 {
            continue;
        }

        let b_row: u64 = st.board.rows[yy as usize];
        let overlap: u16 = if x >= 0 {
            (b_row >> x) as u16
        } else {
            (b_row as u16) << (-x)
        };

        if overlap & row != 0 {
            return false;
        }
    }

    true
}

pub fn try_rotate<const W: usize, const H: usize>(st: &mut State<W, H>, to: Rot) -> bool {
    let kicks = &srs::srs_table()[st.piece as usize][st.rot as usize][to as usize];

    for kick in kicks {
        let nx = st.x + kick.dx;
        let ny = st.y - kick.dy;

        if can_place(st, nx, ny, to) {
            st.x = nx;
            st.y = ny;
            st.rot = to;
            return true;
        }
    }
    false
}

pub fn try_move<const W: usize, const H: usize>(st: &mut State<W, H>, dx: i8, dy: i8) -> bool {
    if can_place(st, st.x + dx, st.y + dy, st.rot) {
        st.x += dx;
        st.y += dy;
        return true;
    }
    false
}

pub fn hard_drop<const W: usize, const H: usize>(st: &mut State<W, H>) -> i32 {
    let mut dist = 0;
    while can_place(st, st.x, st.y + 1, st.rot) {
        st.y += 1;
        dist += 1;
    }
    dist
}

pub fn lock_piece<const W: usize, const H: usize>(st: &mut State<W, H>) -> i32 {
    let shape = &PIECES[st.piece as usize].rot[st.rot as usize];

    for i in 0..4 {
        let row = shape.row[i];
        if row == 0 {
            continue;
        }

        let yy = st.y + i as i8;
        if yy < 0 || yy >= H as i8 {
            continue;
        }

        if st.x >= 0 {
            st.board.rows[yy as usize] |= ((row as u64) << st.x) & Board::<W, H>::FULL;
        } else {
            st.board.rows[yy as usize] |= ((row as u64) >> (-st.x)) & Board::<W, H>::FULL;
        }
    }

    let res = st.board.clear_lines();
    st.last_clear_mask = res.mask;
    st.last_clear_count = res.count;
    res.count as i32
}

pub fn get_ghost_y<const W: usize, const H: usize>(st: &State<W, H>) -> i32 {
    let mut gy = st.y as i32;
    while can_place(st, st.x, (gy + 1) as i8, st.rot) {
        gy += 1;
    }
    gy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::state::State;
    use crate::types::{Piece, Rot};

    fn make_state<const W: usize, const H: usize>(piece: Piece) -> State<W, H> {
        State {
            board: Board::new(),
            piece,
            rot: Rot::R0,
            x: (W as i8) / 2 - 2,
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
    fn test_can_place_spawn() {
        let st = make_state::<10, 20>(Piece::T);
        assert!(can_place(&st, st.x, st.y, st.rot));
    }

    #[test]
    fn test_can_place_left_wall() {
        let st = make_state::<10, 20>(Piece::I);
        assert!(can_place(&st, st.x, st.y, st.rot));
        assert!(!can_place(&st, -2, st.y, st.rot));
    }

    #[test]
    fn test_can_place_right_wall() {
        let st = make_state::<10, 20>(Piece::I);
        assert!(!can_place(&st, 8, st.y, st.rot));
    }

    #[test]
    fn test_can_place_floor() {
        let st = make_state::<10, 20>(Piece::T);
        assert!(!can_place(&st, st.x, 20, st.rot));
    }

    #[test]
    fn test_can_place_occupied() {
        let mut st = make_state::<10, 20>(Piece::T);
        st.board.rows[19] = Board::<10, 20>::FULL;
        assert!(!can_place(&st, st.x, 18, st.rot));
        assert!(can_place(&st, st.x, 17, st.rot));
    }

    #[test]
    fn test_try_move_left() {
        let mut st = make_state::<10, 20>(Piece::T);
        let x0 = st.x;
        assert!(try_move(&mut st, -1, 0));
        assert_eq!(st.x, x0 - 1);
    }

    #[test]
    fn test_try_move_right() {
        let mut st = make_state::<10, 20>(Piece::T);
        let x0 = st.x;
        assert!(try_move(&mut st, 1, 0));
        assert_eq!(st.x, x0 + 1);
    }

    #[test]
    fn test_try_move_soft_drop() {
        let mut st = make_state::<10, 20>(Piece::T);
        assert!(try_move(&mut st, 0, 1));
        assert_eq!(st.y, 1);
    }

    #[test]
    fn test_try_move_left_wall() {
        let mut st = make_state::<10, 20>(Piece::I);
        while try_move(&mut st, -1, 0) {}
        let x_before = st.x;
        let y_before = st.y;
        assert!(!try_move(&mut st, -1, 0));
        assert_eq!(st.x, x_before);
        assert_eq!(st.y, y_before);
    }

    #[test]
    fn test_try_move_occupied_below() {
        let mut st = make_state::<10, 20>(Piece::T);
        st.y = 18;
        st.board.rows[19] = Board::<10, 20>::FULL;
        assert!(!try_move(&mut st, 0, 1));
        assert_eq!(st.y, 18);
    }

    #[test]
    fn test_try_rotate_cw_t() {
        let mut st = make_state::<10, 20>(Piece::T);
        assert!(try_rotate(&mut st, Rot::R90));
        assert_eq!(st.rot, Rot::R90);
    }

    #[test]
    fn test_try_rotate_cw_o() {
        let mut st = make_state::<10, 20>(Piece::O);
        assert!(try_rotate(&mut st, Rot::R90));
        assert_eq!(st.rot, Rot::R90);
    }

    #[test]
    fn test_try_rotate_ccw_t() {
        let mut st = make_state::<10, 20>(Piece::T);
        assert!(try_rotate(&mut st, Rot::R270));
        assert_eq!(st.rot, Rot::R270);
    }

    #[test]
    fn test_try_rotate_all_pieces() {
        let pieces = [
            Piece::I,
            Piece::O,
            Piece::T,
            Piece::S,
            Piece::Z,
            Piece::J,
            Piece::L,
        ];
        for &piece in &pieces {
            let mut st = make_state::<10, 20>(piece);
            assert!(try_rotate(&mut st, Rot::R90), "failed for {:?}", piece);
            assert_eq!(st.rot, Rot::R90);
        }
    }

    #[test]
    fn test_try_rotate_i_near_wall() {
        let mut st = make_state::<10, 20>(Piece::I);
        st.x = 0;
        st.rot = Rot::R0;
        let result = try_rotate(&mut st, Rot::R90);
        if result {
            assert_eq!(st.rot, Rot::R90);
        }
    }

    #[test]
    fn test_lock_piece_no_clear() {
        let mut st = make_state::<10, 20>(Piece::T);
        st.y = 17;
        let cleared = lock_piece(&mut st);
        assert_eq!(cleared, 0);
        let has_cells = st.board.rows.iter().any(|&r| r != 0);
        assert!(has_cells);
    }

    #[test]
    fn test_lock_piece_clear_line() {
        let mut st = make_state::<10, 20>(Piece::I);
        st.y = 18;
        st.rot = Rot::R0;
        for col in 0..10u64 {
            if col < 3 || col > 6 {
                st.board.rows[19] |= 1u64 << col;
            }
        }
        let cleared = lock_piece(&mut st);
        assert_eq!(cleared, 1);
        assert_eq!(st.board.rows[19], 0);
    }

    #[test]
    fn test_hard_drop_empty_board() {
        let mut st = make_state::<10, 20>(Piece::T);
        let dist = hard_drop(&mut st);
        assert!(dist > 0);
        assert!(st.y >= 17);
        assert!(!can_place(&st, st.x, st.y + 1, st.rot));
    }

    #[test]
    fn test_hard_drop_above_occupied() {
        let mut st = make_state::<10, 20>(Piece::T);
        st.board.rows[19] = 0xFF;
        let dist = hard_drop(&mut st);
        assert!(dist > 0);
        assert!(!can_place(&st, st.x, st.y + 1, st.rot));
    }

    #[test]
    fn test_ghost_y_matches_hard_drop() {
        let st1 = make_state::<10, 20>(Piece::T);
        let mut st2 = st1.clone();
        let ghost_y = get_ghost_y(&st1);
        hard_drop(&mut st2);
        assert_eq!(ghost_y, st2.y as i32);
    }

    #[test]
    fn test_ghost_y_above_occupied() {
        let mut st = make_state::<10, 20>(Piece::T);
        st.board.rows[19] = 0xFF;
        let ghost_y = get_ghost_y(&st);
        assert!(ghost_y < 19);
        assert!(!can_place(&st, st.x, (ghost_y + 1) as i8, st.rot));
    }
}

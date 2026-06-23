use serde::Serialize;
use tetris_core::state::State;

/// Grid cell value for inserted garbage cells. Sits above the active-piece
/// range (3..=9) so renderers can apply the dedicated red-border/gray-fill
/// style without colliding with empty (0), locked (1), ghost (2), or piece
/// values.
pub const GARBAGE_CELL: u8 = 10;

#[derive(Serialize)]
#[allow(dead_code)]
pub struct HardDropInfo {
    pub cols: u16,
    pub start_y: i8,
    pub end_y: i8,
    pub piece: u8,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct HudData {
    pub score: u64,
    pub level: u32,
    pub lines: u32,
    pub combo: u32,
    pub b2b: u32,
    pub tspin: u32,
    pub all_clear: u32,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct GameStats {
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    pub game_time_ms: u64,
    pub max_combo: u32,
    pub tspin_count: u32,
    pub total_pieces: u32,
}

#[allow(dead_code)]
pub fn build_grid(state: &State<10, 20>, ghost_y: i32, game_over: bool) -> [u8; 200] {
    let mut grid = [0u8; 200];
    fill_grid_buf(state, ghost_y, game_over, &mut grid);
    grid
}

#[allow(dead_code)]
pub fn fill_grid_buf(state: &State<10, 20>, ghost_y: i32, game_over: bool, buf: &mut [u8]) {
    buf.iter_mut().for_each(|b| *b = 0);

    for y in 0..20usize {
        for x in 0..10usize {
            if state.board.rows[y] & (1u64 << x) != 0 {
                buf[y * 10 + x] = if state.board.garbage[y] & (1u64 << x) != 0 {
                    GARBAGE_CELL
                } else {
                    1
                };
            }
        }
    }

    if !game_over {
        let shape = tetris_core::piece::PIECES[state.piece as usize].rot[state.rot as usize];
        let piece_id = state.piece as u8 + 3;

        for i in 0..4 {
            for j in 0..4 {
                if shape.row[i] & (1 << j) == 0 {
                    continue;
                }
                let xx = state.x + j as i8;
                let yy = state.y + i as i8;
                if !(0..10).contains(&xx) || !(0..20).contains(&yy) {
                    continue;
                }
                buf[yy as usize * 10 + xx as usize] = piece_id;
            }
        }

        if ghost_y >= 0 {
            let gy = ghost_y as usize;
            for i in 0..4 {
                for j in 0..4 {
                    if shape.row[i] & (1 << j) == 0 {
                        continue;
                    }
                    let xx = state.x + j as i8;
                    let yy = gy as i8 + i as i8;
                    if !(0..10).contains(&xx) || !(0..20).contains(&yy) {
                        continue;
                    }
                    if buf[yy as usize * 10 + xx as usize] == 0 {
                        buf[yy as usize * 10 + xx as usize] = 2;
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn build_next(state: &State<10, 20>) -> [u8; 5] {
    let mut next = [0u8; 5];
    for (i, item) in next.iter_mut().enumerate() {
        *item = state.next[i] as u8;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::engine::Engine;
    use tetris_core::rules::get_ghost_y;

    fn test_engine() -> Engine<10, 20> {
        let mut e = Engine::<10, 20>::new();
        e.reset(42);
        e
    }

    #[test]
    fn test_build_grid_length() {
        let e = test_engine();
        let ghost_y = get_ghost_y(&e.state) as i32;
        let grid = build_grid(&e.state, ghost_y, false);
        assert_eq!(grid.len(), 200);
    }

    #[test]
    fn test_build_grid_has_active_piece() {
        let e = test_engine();
        let ghost_y = get_ghost_y(&e.state) as i32;
        let grid = build_grid(&e.state, ghost_y, false);
        let active_count = grid.iter().filter(|&&v| v >= 3).count();
        assert!(
            active_count > 0,
            "active piece should produce non-zero cells"
        );
    }

    #[test]
    fn test_build_grid_game_over_no_active() {
        let e = test_engine();
        let grid = build_grid(&e.state, -1, true);
        let active_count = grid.iter().filter(|&&v| v >= 3).count();
        assert_eq!(active_count, 0, "game over should hide active piece");
    }

    #[test]
    fn test_fill_grid_buf_matches_build_grid() {
        let e = test_engine();
        let ghost_y = get_ghost_y(&e.state) as i32;
        let expected = build_grid(&e.state, ghost_y, false);
        let mut buf = vec![0u8; 200];
        fill_grid_buf(&e.state, ghost_y, false, &mut buf);
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn test_build_grid_marks_garbage_cells() {
        let mut e = test_engine();
        e.state.board.insert_garbage(2, 3);
        let grid = build_grid(&e.state, -1, true);
        // Bottom row (y=19), hole at x=3 stays empty, rest are garbage.
        assert_eq!(grid[19 * 10 + 3], 0, "garbage hole stays empty");
        for x in 0..10 {
            if x == 3 {
                continue;
            }
            assert_eq!(
                grid[19 * 10 + x],
                GARBAGE_CELL,
                "garbage cell at x={x} should use the dedicated value"
            );
        }
    }

    #[test]
    fn test_build_next_returns_5_pieces() {
        let e = test_engine();
        let next = build_next(&e.state);
        assert_eq!(next.len(), 5);
        for &p in &next {
            assert!(p <= 6, "piece ID {p} out of range 0-6");
        }
    }

    #[test]
    fn test_build_next_deterministic() {
        let e1 = test_engine();
        let e2 = test_engine();
        assert_eq!(build_next(&e1.state), build_next(&e2.state));
    }
}

use serde::Serialize;
use tetris_core::state::State;

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
    pub score: u32,
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
    pub score: u32,
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
                buf[y * 10 + x] = 1;
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

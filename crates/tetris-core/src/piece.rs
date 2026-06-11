#[derive(Debug, Clone, Copy)]
pub struct Shape {
    pub row: [u16; 4],
}

#[derive(Debug, Clone, Copy)]
pub struct PieceDef {
    pub rot: [Shape; 4],
}

pub const fn rot90_4x4(x: u16) -> u16 {
    let mut r: u16 = 0;
    let mut y = 0;
    while y < 4 {
        let mut x2 = 0;
        while x2 < 4 {
            let src = y * 4 + x2;
            let dst = x2 * 4 + (3 - y);
            if (x & (1 << src)) != 0 {
                r |= 1 << dst;
            }
            x2 += 1;
        }
        y += 1;
    }
    r
}

pub const fn rot90_3x3(x: u16) -> u16 {
    let mut r: u16 = 0;
    let mut y = 0;
    while y < 3 {
        let mut x2 = 0;
        while x2 < 3 {
            let src = y * 4 + x2;
            let dst = x2 * 4 + (2 - y);
            if (x & (1 << src)) != 0 {
                r |= 1 << dst;
            }
            x2 += 1;
        }
        y += 1;
    }
    r
}

pub const fn to_rows(shape: u16) -> Shape {
    let mut r = Shape { row: [0; 4] };
    let mut y = 0;
    while y < 4 {
        let mut row: u16 = 0;
        let mut x = 0;
        while x < 4 {
            if (shape & (1 << (y * 4 + x))) != 0 {
                row |= 1 << x;
            }
            x += 1;
        }
        r.row[y] = row;
        y += 1;
    }
    r
}

pub const fn make_piece_4x4(base: u16) -> PieceDef {
    let r1 = rot90_4x4(base);
    let r2 = rot90_4x4(r1);
    let r3 = rot90_4x4(r2);
    PieceDef {
        rot: [to_rows(base), to_rows(r1), to_rows(r2), to_rows(r3)],
    }
}

pub const fn make_piece_3x3(base: u16) -> PieceDef {
    let r1 = rot90_3x3(base);
    let r2 = rot90_3x3(r1);
    let r3 = rot90_3x3(r2);
    PieceDef {
        rot: [to_rows(base), to_rows(r1), to_rows(r2), to_rows(r3)],
    }
}

pub const PIECES: [PieceDef; 7] = [
    make_piece_4x4(0x00F0), // I
    make_piece_4x4(0x0660), // O
    make_piece_3x3(0x0072), // T
    make_piece_3x3(0x0036), // S
    make_piece_3x3(0x0063), // Z
    make_piece_3x3(0x0071), // J
    make_piece_3x3(0x0074), // L
];

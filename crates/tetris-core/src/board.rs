use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct Board<const W: usize, const H: usize> {
    pub rows: [u64; H],
}

impl<const W: usize, const H: usize> Serialize for Board<W, H> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTuple;
        let mut tup = serializer.serialize_tuple(H)?;
        for row in &self.rows {
            tup.serialize_element(row)?;
        }
        tup.end()
    }
}

impl<'de, const W: usize, const H: usize> Deserialize<'de> for Board<W, H> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BoardVisitor<const W: usize, const H: usize>;
        impl<'de, const W: usize, const H: usize> serde::de::Visitor<'de> for BoardVisitor<W, H> {
            type Value = Board<W, H>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(&format!("a tuple of {H} u64s"))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Self::Value, A::Error> {
                let mut rows = [0u64; H];
                for (i, row) in rows.iter_mut().enumerate() {
                    *row = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Board { rows })
            }
        }
        deserializer.deserialize_tuple(H, BoardVisitor::<W, H>)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClearResult {
    pub mask: u64,
    pub count: u8,
}

impl<const W: usize, const H: usize> Board<W, H> {
    pub const FULL: u64 = {
        assert!(W < 64);
        (1u64 << W) - 1
    };

    pub fn new() -> Self {
        Board { rows: [0; H] }
    }

    pub fn collide(&self, y: u8, mask: u64) -> bool {
        self.rows[y as usize] & mask != 0
    }

    pub fn place(&mut self, y: u8, mask: u64) {
        // Mask off any bits beyond board width (defensive; current callers pass
        // in-range masks). Keeps stored rows consistent with `FULL` semantics.
        self.rows[y as usize] |= mask & Self::FULL;
    }

    pub fn full(&self, y: u8) -> bool {
        self.rows[y as usize] == Self::FULL
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|&r| r == 0)
    }

    pub fn column_heights(&self) -> [u8; W] {
        let mut heights = [0u8; W];
        for (col, height) in heights.iter_mut().enumerate() {
            *height = (0..H)
                .find(|&row| self.rows[row] & (1u64 << col) != 0)
                .map_or(0, |row| (H - row) as u8);
        }
        heights
    }

    pub fn holes(&self) -> u32 {
        let mut holes = 0;
        for col in 0..W {
            let mut seen_block = false;
            for row in 0..H {
                if self.rows[row] & (1u64 << col) != 0 {
                    seen_block = true;
                } else if seen_block {
                    holes += 1;
                }
            }
        }
        holes
    }

    pub fn aggregate_height(&self) -> u32 {
        self.column_heights()
            .iter()
            .map(|&height| u32::from(height))
            .sum()
    }

    pub fn bumpiness(&self) -> u32 {
        self.column_heights()
            .windows(2)
            .map(|cols| u32::from(cols[0].abs_diff(cols[1])))
            .sum()
    }

    pub fn wells(&self) -> u32 {
        let heights = self.column_heights();
        let mut wells = 0;
        for col in 0..W {
            let left = if col == 0 { H as u8 } else { heights[col - 1] };
            let right = if col + 1 == W {
                H as u8
            } else {
                heights[col + 1]
            };
            let rim = left.min(right);
            if rim > heights[col] {
                wells += u32::from(rim - heights[col]);
            }
        }
        wells
    }

    pub fn row_transitions(&self) -> u32 {
        let mut transitions = 0;
        for row in self.rows {
            if row == 0 {
                continue;
            }

            let mut previous_filled = true;
            for col in 0..W {
                let filled = row & (1u64 << col) != 0;
                if filled != previous_filled {
                    transitions += 1;
                }
                previous_filled = filled;
            }

            if !previous_filled {
                transitions += 1;
            }
        }
        transitions
    }

    pub fn covered_holes(&self) -> u32 {
        let mut covered = 0;
        for col in 0..W {
            let mut filled_above = 0;
            for row in 0..H {
                if self.rows[row] & (1u64 << col) != 0 {
                    filled_above += 1;
                } else if filled_above > 0 {
                    covered += filled_above;
                }
            }
        }
        covered
    }

    pub fn clear_lines(&mut self) -> ClearResult {
        let mut write = H;
        let mut cleared: u8 = 0;
        let mut mask: u64 = 0;

        for read in (0..H).rev() {
            if self.rows[read] == Self::FULL {
                cleared += 1;
                mask |= 1u64 << read;
            } else {
                write = write.wrapping_sub(1);
                self.rows[write] = self.rows[read];
            }
        }

        for y in 0..write {
            self.rows[y] = 0;
        }

        ClearResult {
            mask,
            count: cleared,
        }
    }

    pub fn insert_garbage(&mut self, lines: u8, hole_x: u8) {
        if lines == 0 {
            return;
        }
        let hole_x = (hole_x as usize).min(W - 1);
        let l = (lines as usize).min(H);
        for y in 0..(H - l) {
            self.rows[y] = self.rows[y + l];
        }
        let garbage_row = Self::FULL & !(1u64 << hole_x);
        for y in (H - l)..H {
            self.rows[y] = garbage_row;
        }
    }
}

impl<const W: usize, const H: usize> Default for Board<W, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize, const H: usize> crate::traits::TetrisBoard for Board<W, H> {
    const FULL: u64 = Board::<W, H>::FULL;

    fn new() -> Self {
        Board::new()
    }

    fn collide(&self, y: u8, mask: u64) -> bool {
        self.collide(y, mask)
    }

    fn place(&mut self, y: u8, mask: u64) {
        self.place(y, mask);
    }

    fn full(&self, y: u8) -> bool {
        self.full(y)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn clear_lines(&mut self) -> ClearResult {
        self.clear_lines()
    }

    fn insert_garbage(&mut self, lines: u8, hole_x: u8) {
        self.insert_garbage(lines, hole_x);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_lines_single_full_line() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = Board::<10, 20>::FULL;
        let result = board.clear_lines();
        assert_eq!(result.count, 1);
        assert_eq!(result.mask, 1u64 << 19);
        assert_eq!(board.rows[19], 0);
    }

    #[test]
    fn test_clear_lines_multiple_with_gaps() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = Board::<10, 20>::FULL;
        board.rows[18] = 0x155;
        board.rows[17] = Board::<10, 20>::FULL;
        let result = board.clear_lines();
        assert_eq!(result.count, 2);
        assert_eq!(board.rows[19], 0x155);
        assert_eq!(board.rows[18], 0);
        assert_eq!(board.rows[17], 0);
    }

    #[test]
    fn test_clear_lines_no_full_lines() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = 0x001;
        board.rows[18] = 0x002;
        let result = board.clear_lines();
        assert_eq!(result.count, 0);
        assert_eq!(board.rows[19], 0x001);
        assert_eq!(board.rows[18], 0x002);
    }

    #[test]
    fn test_clear_lines_full_board() {
        let mut board = Board::<10, 20>::new();
        for i in 0..20 {
            board.rows[i] = Board::<10, 20>::FULL;
        }
        let result = board.clear_lines();
        assert_eq!(result.count, 20);
        for i in 0..20 {
            assert_eq!(board.rows[i], 0);
        }
    }

    #[test]
    fn test_clear_lines_empty_board() {
        let mut board = Board::<10, 20>::new();
        let result = board.clear_lines();
        assert_eq!(result.count, 0);
    }

    #[test]
    fn test_insert_garbage_3_lines_hole_4() {
        let mut board = Board::<10, 20>::new();
        board.insert_garbage(3, 4);
        let garbage_row = Board::<10, 20>::FULL & !(1u64 << 4);
        for i in 0..17 {
            assert_eq!(board.rows[i], 0);
        }
        assert_eq!(board.rows[19], garbage_row);
        assert_eq!(board.rows[18], garbage_row);
        assert_eq!(board.rows[17], garbage_row);
    }

    #[test]
    fn column_heights_empty_board_returns_zeroes() {
        let board = Board::<10, 20>::new();
        assert_eq!(board.column_heights(), [0; 10]);
    }

    #[test]
    fn aggregate_height_full_bottom_row_returns_width() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = Board::<10, 20>::FULL;
        assert_eq!(board.aggregate_height(), 10);
    }

    #[test]
    fn holes_counts_empty_cells_below_first_block() {
        let mut board = Board::<10, 20>::new();
        board.rows[17] = 1;
        board.rows[19] = 1;
        assert_eq!(board.holes(), 1);
    }

    #[test]
    fn bumpiness_sums_adjacent_height_deltas() {
        let mut board = Board::<10, 20>::new();
        board.rows[18] = 1;
        board.rows[19] = 0b11;
        assert_eq!(board.bumpiness(), 2);
    }

    #[test]
    fn wells_counts_depression_depth() {
        let mut board = Board::<10, 20>::new();
        board.rows[17] = 0b101;
        board.rows[18] = 0b101;
        board.rows[19] = 0b101;
        assert_eq!(board.wells(), 3);
    }

    #[test]
    fn row_transitions_empty_board_returns_zero() {
        let board = Board::<10, 20>::new();
        assert_eq!(board.row_transitions(), 0);
    }

    #[test]
    fn row_transitions_counts_single_mid_row_cell_with_walls() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = 1u64 << 4;
        assert_eq!(board.row_transitions(), 4);
    }

    #[test]
    fn covered_holes_empty_board_returns_zero() {
        let board = Board::<10, 20>::new();
        assert_eq!(board.covered_holes(), 0);
    }

    #[test]
    fn covered_holes_counts_blocks_above_hole() {
        let mut board = Board::<10, 20>::new();
        board.rows[16] = 1;
        board.rows[17] = 1;
        board.rows[18] = 1;
        assert_eq!(board.covered_holes(), 3);
    }
}

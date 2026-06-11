#[derive(Debug, Clone)]
pub struct Board<const W: usize, const H: usize> {
    pub rows: [u64; H],
}

#[derive(Debug, Clone, Copy)]
pub struct ClearResult {
    pub mask: u32,
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
        self.rows[y as usize] |= mask;
    }

    pub fn full(&self, y: u8) -> bool {
        self.rows[y as usize] == Self::FULL
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|&r| r == 0)
    }

    pub fn clear_lines(&mut self) -> ClearResult {
        let mut write = H;
        let mut cleared: u8 = 0;
        let mut mask: u32 = 0;

        for read in (0..H).rev() {
            if self.rows[read] != Self::FULL {
                write = write.wrapping_sub(1);
                self.rows[write] = self.rows[read];
            } else {
                cleared += 1;
                mask |= 1u32 << read;
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
        let l = lines as usize;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_lines_single_full_line() {
        let mut board = Board::<10, 20>::new();
        board.rows[19] = Board::<10, 20>::FULL;
        let result = board.clear_lines();
        assert_eq!(result.count, 1);
        assert_eq!(result.mask, 1u32 << 19);
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
}

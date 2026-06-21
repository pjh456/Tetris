use tetris_core::scoring::ScoreTracker;

#[test]
fn test_single_clear_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(1, false, false, false, false, 0, 0, 0, 1);
    assert_eq!(s.score, 100);
}

#[test]
fn test_double_clear_level_2() {
    let mut s = ScoreTracker::default();
    s.level = 2;
    s.update(2, false, false, false, false, 0, 0, 0, 2);
    assert_eq!(s.score, 600);
}

#[test]
fn test_tspin_double_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(2, true, false, false, false, 0, 0, 0, 1);
    assert_eq!(s.score, 1200);
}

#[test]
fn test_b2b_tetris_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(4, false, false, true, false, 0, 0, 0, 1);
    assert_eq!(s.score, 1200);
}

#[test]
fn test_combo_3_singles_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(1, false, false, false, false, 0, 0, 0, 1);
    assert_eq!(s.score, 100);
    s.update(1, false, false, false, false, 0, 0, 1, 1);
    assert_eq!(s.score, 250);
    s.update(1, false, false, false, false, 0, 0, 2, 1);
    assert_eq!(s.score, 450);
}

#[test]
fn test_perfect_clear_single_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(1, false, false, false, true, 0, 0, 0, 1);
    assert_eq!(s.score, 900);
}

#[test]
fn test_level_cap_at_15() {
    let mut s = ScoreTracker::default();
    s.total_lines = 140;
    s.level = 15;
    s.update(4, false, false, false, false, 0, 0, 0, 15);
    assert_eq!(s.level, 15);
}

#[test]
fn test_hard_drop_10_cells() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(0, false, false, false, false, 0, 10, 0, 1);
    assert_eq!(s.score, 20);
}

#[test]
fn test_tspin_mini_single_level_1() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(1, true, true, false, false, 0, 0, 0, 1);
    assert_eq!(s.score, 200);
}

#[test]
fn test_tspin_mini_double_level_1() {
    // Tetris guideline: T-Spin Mini Double = 400 (was an oversight at flat 200).
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(2, true, true, false, false, 0, 0, 0, 1);
    assert_eq!(s.score, 400);
}

#[test]
fn test_b2b_chain_broken() {
    let mut s = ScoreTracker::default();
    s.level = 1;
    s.update(4, false, false, true, false, 0, 0, 0, 1);
    assert_eq!(s.score, 1200);
    s.update(1, false, false, false, false, 0, 0, 1, 1);
    assert_eq!(s.score, 1200 + 100 + 50);
    s.update(4, false, false, false, false, 0, 0, 2, 1);
    assert_eq!(s.score, 1200 + 150 + 800 + 100);
}

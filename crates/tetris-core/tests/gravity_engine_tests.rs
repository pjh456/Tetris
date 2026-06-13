use tetris_core::engine::Engine;

#[test]
fn test_gravity_interval_level_1() {
    use tetris_core::engine::gravity_interval_ms;
    assert_eq!(gravity_interval_ms(1), 800);
}

#[test]
fn test_gravity_interval_level_15_fast() {
    use tetris_core::engine::gravity_interval_ms;
    assert!(gravity_interval_ms(15) <= 8);
}

#[test]
fn test_tick_50x16ms_level1_one_drop() {
    let mut e = Engine::<10, 20>::new();
    e.reset_with_level(42, 1);
    let start_y = e.state.y;
    for _ in 0..50 {
        e.tick(16);
    }
    assert_eq!(e.state.y, start_y + 1);
}

#[test]
fn test_tick_8ms_level15_drop_occurs() {
    let mut e = Engine::<10, 20>::new();
    e.reset_with_level(42, 15);
    let start_y = e.state.y;
    e.tick(8);
    assert!(e.state.y > start_y);
}

#[test]
fn test_tick_1000ms_level1_one_drop_with_remainder() {
    let mut e = Engine::<10, 20>::new();
    e.reset_with_level(42, 1);
    let start_y = e.state.y;
    e.tick(1000);
    assert_eq!(e.state.y, start_y + 1);
}

#[test]
fn test_determinism_same_seed() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset_with_level(99, 1);
    b.reset_with_level(99, 1);
    for _ in 0..500 {
        a.tick(16);
        b.tick(16);
    }
    assert_eq!(a.state.board.rows, b.state.board.rows);
    assert_eq!(a.state.piece, b.state.piece);
}

#[test]
fn test_tick_zero_no_gravity() {
    let mut e = Engine::<10, 20>::new();
    e.reset_with_level(42, 1);
    let start_y = e.state.y;
    e.tick(0);
    assert_eq!(e.state.y, start_y);
}

#[test]
fn test_game_over_tick_returns_default() {
    let mut e = Engine::<10, 20>::new();
    e.reset_with_level(42, 1);
    e.game_over = true;
    let res = e.tick(100);
    assert_eq!(res.damage, 0);
    assert!(!res.is_tspin);
}

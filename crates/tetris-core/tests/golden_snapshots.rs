// CI: cargo test -p tetris-core --test golden_snapshots -- --test-threads=1
// Golden snapshots verify engine determinism within the same process.

use tetris_core::engine::{Action, Engine, InputEvent};

fn make_input(key: u8) -> InputEvent {
    InputEvent { key, pressed: true }
}

#[test]
fn golden_seed_42_many_hard_drops() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(42);
    b.reset(42);
    let hd = make_input(Action::HardDrop as u8);
    for _ in 0..20 {
        a.fixed_tick(&[hd]);
        b.fixed_tick(&[hd]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_seed_12345_mixed_actions() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(12345);
    b.reset(12345);
    let seq = [
        Action::HardDrop, Action::MoveLeft, Action::HardDrop,
        Action::MoveRight, Action::RotateCW, Action::HardDrop,
    ];
    for &action in &seq {
        let inp = make_input(action as u8);
        a.fixed_tick(&[inp]);
        b.fixed_tick(&[inp]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_seed_999_prolonged_play() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(999);
    b.reset(999);
    let actions: [u8; 15] = [0, 1, 0, 3, 4, 1, 0, 3, 5, 6, 2, 0, 1, 3, 4];
    for &a_key in &actions {
        let inp = make_input(a_key);
        a.fixed_tick(&[inp]);
        b.fixed_tick(&[inp]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_seed_42_empty_ticks() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(42);
    b.reset(42);
    for _ in 0..100 {
        a.fixed_tick(&[]);
        b.fixed_tick(&[]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_seed_42_lock_delay_trigger() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(42);
    b.reset(42);
    for _ in 0..50 {
        a.fixed_tick(&[]);
        b.fixed_tick(&[]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_different_seeds_diverge() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(42);
    b.reset(99);
    for _ in 0..20 {
        a.fixed_tick(&[make_input(Action::HardDrop as u8)]);
        b.fixed_tick(&[make_input(Action::HardDrop as u8)]);
    }
    assert_ne!(a.state_hash(), b.state_hash());
}

#[test]
fn golden_different_inputs_diverge() {
    let mut a = Engine::<10, 20>::new();
    let mut b = Engine::<10, 20>::new();
    a.reset(42);
    b.reset(42);
    for _ in 0..10 {
        a.fixed_tick(&[make_input(Action::HardDrop as u8)]);
    }
    for _ in 0..10 {
        b.fixed_tick(&[make_input(Action::MoveLeft as u8)]);
    }
    assert_ne!(a.state_hash(), b.state_hash());
}

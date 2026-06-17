use proptest::prelude::*;
use proptest::test_runner::Config;
use tetris_core::engine::{Engine, InputEvent};

fn make_input_event(key: u8) -> InputEvent {
    InputEvent { key, pressed: true }
}

proptest! {
    #![proptest_config(Config::with_cases(1000))]

    #[test]
    fn same_seed_same_input_same_hash(
        seed in 0u32..1_000_000,
        actions in proptest::collection::vec(0u8..7u8, 0..200),
    ) {
        let mut engine_a = Engine::<10, 20>::new();
        let mut engine_b = Engine::<10, 20>::new();
        engine_a.reset(seed);
        engine_b.reset(seed);

        for chunk in actions.chunks(10) {
            let inputs: Vec<InputEvent> = chunk
                .iter()
                .map(|&a| make_input_event(a))
                .collect();
            engine_a.fixed_tick(&inputs);
            engine_b.fixed_tick(&inputs);
            prop_assert_eq!(engine_a.state_hash(), engine_b.state_hash());
        }
    }

    #[test]
    fn different_seed_different_hash(
        seed_a in 0u32..500_000,
        seed_b in 500_001u32..1_000_000,
    ) {
        let mut engine_a = Engine::<10, 20>::new();
        let mut engine_b = Engine::<10, 20>::new();
        engine_a.reset(seed_a);
        engine_b.reset(seed_b);

        let inputs: Vec<InputEvent> = (0..50)
            .map(|_| make_input_event(3))
            .collect();

        engine_a.fixed_tick(&inputs);
        engine_b.fixed_tick(&inputs);

        prop_assert_ne!(engine_a.state_hash(), engine_b.state_hash());
    }
}

// WR-21 grounded-only lock delay: in-air actions (including cancelling pairs like
// MoveLeft+MoveRight) produce identical state. Different inputs CAN produce same hash
// — correct behavior, not a bug. The property "different input -> different hash"
// does not hold in general with grounded-only lock delay.
#[test]
fn different_input_same_hash_is_expected() {
    let mut engine_a = Engine::<10, 20>::new();
    let mut engine_b = Engine::<10, 20>::new();
    engine_a.reset(0);
    engine_b.reset(0);
    // MoveLeft then MoveRight at spawn = no net change, same state
    engine_a.fixed_tick(&[make_input_event(0), make_input_event(1)]);
    engine_a.fixed_tick(&[]);
    engine_b.fixed_tick(&[]);
    engine_b.fixed_tick(&[]);
    assert_eq!(engine_a.state_hash(), engine_b.state_hash());
}

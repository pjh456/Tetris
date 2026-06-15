use tetris_core::engine::{Engine, InputEvent};
use proptest::prelude::*;
use proptest::test_runner::Config;

fn make_input_event(key: u8) -> InputEvent {
    InputEvent {
        key,
        pressed: true,
    }
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

        // Apply the same sequence and verify divergence
        let inputs: Vec<InputEvent> = (0..50)
            .map(|_| make_input_event(3)) // HardDrop × 50
            .collect();

        engine_a.fixed_tick(&inputs);
        engine_b.fixed_tick(&inputs);

        prop_assert_ne!(engine_a.state_hash(), engine_b.state_hash());
    }

    #[test]
    fn different_input_different_hash(
        seed in 0u32..1_000_000,
        extra_actions in proptest::collection::vec(0u8..7u8, 1..50),
    ) {
        let mut engine_a = Engine::<10, 20>::new();
        let mut engine_b = Engine::<10, 20>::new();
        engine_a.reset(seed);
        engine_b.reset(seed);

        // Engine_A: no extra actions, just gravity
        for _ in 0..10 {
            engine_a.fixed_tick(&[]);
        }

        // Engine_B: same ticks but with extra actions mixed in
        for action_chunk in extra_actions.chunks(5) {
            let inputs: Vec<InputEvent> = action_chunk
                .iter()
                .map(|&a| make_input_event(a))
                .collect();
            engine_b.fixed_tick(&inputs);
        }

        prop_assert_ne!(engine_a.state_hash(), engine_b.state_hash());
    }
}

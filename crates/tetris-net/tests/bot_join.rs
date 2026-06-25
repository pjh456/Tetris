use std::time::Duration;

use tetris_core::{Engine, rl};
use tetris_infer::{Layer, MlpPolicy};
use tetris_net::bot::AiBot;
use tetris_net::host_adapter::RenetHostAdapter;
use tetris_net::network_manager::{MODEL_B_CHANNEL, NetworkManager};
use tetris_protocol::newtypes::{PlayerSlot, Seed};

fn zero_policy() -> MlpPolicy {
    MlpPolicy::new(
        rl::OBS_DIM,
        rl::ACTION_SPACE_SIZE,
        vec![Layer {
            weight: vec![vec![0.0; rl::OBS_DIM]; rl::ACTION_SPACE_SIZE],
            bias: vec![0.0; rl::ACTION_SPACE_SIZE],
            norm: None,
        }],
    )
}

#[test]
fn bot_join_sends_replay_that_advances_authority_state_hash() {
    let mut server = NetworkManager::new();
    server.start_server("127.0.0.1", 0, 2).unwrap();
    let addr = server.server_addr().unwrap();

    let mut client = NetworkManager::new();
    client
        .connect_to_server(&addr.ip().to_string(), addr.port())
        .unwrap();

    let mut adapter = RenetHostAdapter::new(Seed(42), 2);
    adapter.start_playing();

    let mut initial_engine = Engine::<10, 20>::new();
    initial_engine.reset(42);
    let before_hash = initial_engine.state_hash();

    let mut bot = AiBot::new(zero_policy(), 42, 0.0);
    let mut sent = false;
    for _ in 0..64 {
        if let Some(replay) = bot.next_replay(1) {
            client.send_packet(&replay, MODEL_B_CHANNEL).unwrap();
            sent = true;
        }
        client.tick(Duration::from_millis(16)).unwrap();
        adapter
            .tick(&mut server, Duration::from_millis(16))
            .unwrap();
    }

    assert!(sent);
    assert_eq!(server.connected_count(), 1);
    assert_ne!(
        adapter.sim.engine(PlayerSlot(1)).unwrap().state_hash(),
        before_hash
    );
}

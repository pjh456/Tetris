use std::fs;
use std::time::Duration;

use tetris_cli::multiplayer::{MultiplayerMode, spawn_ai_opponent};
use tetris_core::{Engine, rl};
use tetris_net::host_adapter::RenetHostAdapter;
use tetris_net::network_manager::NetworkManager;
use tetris_protocol::newtypes::{PlayerSlot, Seed};

#[test]
fn ai_opponent_joins_and_advances_authority_slot() {
    let mut server = NetworkManager::new();
    server.start_server("127.0.0.1", 0, 3).unwrap();
    let addr = server.server_addr().unwrap();

    let test_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("ai_opponent_test");
    let models_dir = test_dir.join("models");
    fs::create_dir_all(&models_dir).unwrap();
    fs::write(models_dir.join("weights.json"), zero_weights_json()).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&test_dir).unwrap();
    let mode = MultiplayerMode::HostP2p { bind_addr: addr };
    let first_handle = spawn_ai_opponent(&mode, 0.0).unwrap();
    let second_handle = spawn_ai_opponent(&mode, 0.0).unwrap();
    std::env::set_current_dir(original_dir).unwrap();

    let mut adapter = RenetHostAdapter::new(Seed(42), 3);
    adapter.start_playing();

    let mut initial_engine = Engine::<10, 20>::new();
    initial_engine.reset(42);
    let before_hash = initial_engine.state_hash();

    for _ in 0..96 {
        adapter
            .tick(&mut server, Duration::from_millis(16))
            .unwrap();
        let first_advanced = adapter
            .sim
            .engine(PlayerSlot(1))
            .is_some_and(|engine| engine.state_hash() != before_hash);
        let second_advanced = adapter
            .sim
            .engine(PlayerSlot(2))
            .is_some_and(|engine| engine.state_hash() != before_hash);
        if first_advanced && second_advanced {
            break;
        }
        std::thread::sleep(Duration::from_millis(4));
    }

    assert!(first_handle.sent_replay_count() > 0);
    assert!(second_handle.sent_replay_count() > 0);
    assert_ne!(
        adapter.sim.engine(PlayerSlot(1)).unwrap().state_hash(),
        before_hash
    );
    assert_ne!(
        adapter.sim.engine(PlayerSlot(2)).unwrap().state_hash(),
        before_hash
    );
    first_handle.stop();
    second_handle.stop();
}

fn zero_weights_json() -> String {
    let mut rows = Vec::with_capacity(rl::ACTION_SPACE_SIZE);
    let row = format!("[{}]", vec!["0.0"; rl::OBS_DIM].join(","));
    for _ in 0..rl::ACTION_SPACE_SIZE {
        rows.push(row.clone());
    }
    format!(
        "{{\"input_dim\":{},\"output_dim\":{},\"activation\":\"tanh\",\"layers\":[{{\"weight\":[{}],\"bias\":[{}]}}]}}",
        rl::OBS_DIM,
        rl::ACTION_SPACE_SIZE,
        rows.join(","),
        vec!["0.0"; rl::ACTION_SPACE_SIZE].join(",")
    )
}

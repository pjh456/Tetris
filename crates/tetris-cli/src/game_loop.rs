use std::time::{Duration, Instant};

use crate::app::{AppState, Message};
use crate::config::CliConfig;
use crate::input::InputHandler;
use crate::multiplayer::{AiOpponentHandle, CliNetworkRuntime, spawn_ai_opponent_for_runtime};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiOpponentConfig {
    pub count: usize,
    pub temperature: f32,
}

pub fn run_game_loop<F, G>(
    mut state: AppState,
    config: &CliConfig,
    ai_opponent: Option<AiOpponentConfig>,
    mut on_update: F,
    mut on_render: G,
) where
    F: FnMut(&mut AppState, Message) -> bool,
    G: FnMut(&mut AppState),
{
    let tick_interval = Duration::from_millis(20);
    let frame_interval = Duration::from_millis(16);
    let mut last_tick = Instant::now();
    let mut last_frame = Instant::now();
    let mut input = InputHandler::new(config);
    let mut network: Option<CliNetworkRuntime> = None;
    let mut ai_handles: Vec<AiOpponentHandle> = Vec::new();

    loop {
        while let Some(msg) = input.poll() {
            if on_update(&mut state, msg) {
                return;
            }
        }

        for msg in input.process_repeats() {
            if on_update(&mut state, msg) {
                return;
            }
        }

        if pump_multiplayer_network(
            &mut state,
            &mut network,
            &mut ai_handles,
            ai_opponent,
            &mut on_update,
        ) {
            return;
        }

        let now = Instant::now();

        while now - last_tick >= tick_interval {
            if on_update(&mut state, Message::Tick) {
                return;
            }
            if pump_multiplayer_network(
                &mut state,
                &mut network,
                &mut ai_handles,
                ai_opponent,
                &mut on_update,
            ) {
                return;
            }
            last_tick += tick_interval;
        }

        if now - last_frame >= frame_interval {
            let _ = on_update(&mut state, Message::FrameTick);
            on_render(&mut state);
            last_frame = now;
        }

        // Busy-wait: yields CPU between frames. ratatui event::poll timeout
        // preferred for production; kept for simplicity in terminal environments.
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn pump_multiplayer_network<F>(
    state: &mut AppState,
    network: &mut Option<CliNetworkRuntime>,
    ai_handles: &mut Vec<AiOpponentHandle>,
    ai_opponent: Option<AiOpponentConfig>,
    on_update: &mut F,
) -> bool
where
    F: FnMut(&mut AppState, Message) -> bool,
{
    let AppState::PlayingMulti { session, .. } = state else {
        *network = None;
        for handle in ai_handles.drain(..) {
            handle.stop();
        }
        return false;
    };

    let key = CliNetworkRuntime::mode_key(&session.mode);
    let needs_connect = network
        .as_ref()
        .is_none_or(|runtime| runtime.key() != key.as_str());
    if needs_connect {
        *network = CliNetworkRuntime::connect(&session.mode).ok();
    }

    if let (Some(config), Some(runtime)) = (ai_opponent, network.as_ref()) {
        while ai_handles.len() < config.count {
            let Ok(handle) =
                spawn_ai_opponent_for_runtime(&session.mode, runtime, config.temperature)
            else {
                break;
            };
            ai_handles.push(handle);
        }
    }

    let inbound = if let Some(runtime) = network {
        runtime.pump(session)
    } else {
        session.status = crate::multiplayer::ConnectionStatus::Disconnected;
        Vec::new()
    };

    for packet in inbound {
        if on_update(state, Message::NetworkPacket(packet)) {
            return true;
        }
    }
    false
}

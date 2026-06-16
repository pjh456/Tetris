use std::time::{Duration, Instant};

use crate::app::{AppState, Message};
use crate::config::CliConfig;
use crate::input::InputHandler;

pub fn run_game_loop<F, G>(
    mut state: AppState,
    config: &CliConfig,
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

        let now = Instant::now();

        while now - last_tick >= tick_interval {
            if on_update(&mut state, Message::Tick) {
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

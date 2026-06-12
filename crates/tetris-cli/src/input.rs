use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::Message;
use crate::config::CliConfig;

struct KeyState {
    pressed_at: Instant,
    last_repeat: Instant,
}

pub struct InputHandler {
    held_keys: HashMap<KeyCode, KeyState>,
    das_ms: u64,
    arr_ms: u64,
}

fn is_repeatable(code: &KeyCode) -> bool {
    matches!(code, KeyCode::Left | KeyCode::Right | KeyCode::Down)
}

impl InputHandler {
    pub fn new(config: &CliConfig) -> Self {
        InputHandler {
            held_keys: HashMap::new(),
            das_ms: config.das_ms as u64,
            arr_ms: config.arr_ms as u64,
        }
    }

    pub fn poll(&mut self) -> Option<Message> {
        if !event::poll(Duration::from_millis(1)).unwrap_or(false) {
            return None;
        }
        let ev = event::read().ok()?;
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                let code = key.code;
                if is_repeatable(&code) {
                    self.held_keys.insert(
                        code,
                        KeyState {
                            pressed_at: Instant::now(),
                            last_repeat: Instant::now(),
                        },
                    );
                }
                Some(Message::Key(code))
            }
            Event::Key(key) if key.kind == KeyEventKind::Release => {
                self.held_keys.remove(&key.code);
                None
            }
            _ => None,
        }
    }

    pub fn process_repeats(&mut self) -> Vec<Message> {
        let now = Instant::now();
        let mut msgs = Vec::new();
        for (key, state) in &mut self.held_keys {
            let elapsed = (now - state.pressed_at).as_millis() as u64;
            if elapsed < self.das_ms {
                continue;
            }
            let since_last = (now - state.last_repeat).as_millis() as u64;
            if self.arr_ms == 0 || since_last >= self.arr_ms {
                msgs.push(Message::Key(*key));
                state.last_repeat = now;
            }
        }
        msgs
    }
}

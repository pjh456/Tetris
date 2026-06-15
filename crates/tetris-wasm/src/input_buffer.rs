use tetris_protocol::newtypes::{KeyAction, TickNumber};
use tetris_protocol::protocol::InputEvent;

const FLUSH_INTERVAL_TICKS: u32 = 30;

/// Client-side input buffer for batching key events per D-04.
pub struct ClientInputBuffer {
    events: Vec<InputEvent>,
    tick: TickNumber,
    last_flush_tick: TickNumber,
}

impl ClientInputBuffer {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            tick: TickNumber(0),
            last_flush_tick: TickNumber(0),
        }
    }

    pub fn push(&mut self, key: KeyAction, pressed: bool, subframe: f32) {
        self.events.push(InputEvent {
            key,
            pressed,
            tick: self.tick,
            subframe,
        });
    }

    pub fn advance_tick(&mut self) {
        self.tick.0 += 1;
    }

    pub fn should_flush(&self) -> bool {
        (self.tick.0 - self.last_flush_tick.0) >= FLUSH_INTERVAL_TICKS as u64
    }

    pub fn flush(&mut self) -> Vec<InputEvent> {
        let drained = std::mem::take(&mut self.events);
        self.last_flush_tick = self.tick;
        drained
    }

    pub fn current_tick(&self) -> TickNumber {
        self.tick
    }
}

impl Default for ClientInputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

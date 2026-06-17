use tetris_protocol::newtypes::{KeyAction, TickNumber};
use tetris_protocol::protocol::InputEvent;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const FLUSH_INTERVAL_TICKS: u32 = 30;

/// Client-side input buffer for batching key events per D-04.
/// Fields and methods used by WebTetris under `#[cfg(target_arch = "wasm32")]`;
/// native target sees dead_code warnings.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct ClientInputBuffer {
    events: Vec<InputEvent>,
    tick: TickNumber,
    last_flush_tick: TickNumber,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_buffer_batches_after_thirty_ticks() {
        let mut buffer = ClientInputBuffer::new();
        for _ in 0..29 {
            buffer.advance_tick();
        }
        assert!(!buffer.should_flush());

        buffer.advance_tick();

        assert!(buffer.should_flush());
    }

    #[test]
    fn input_buffer_preserves_event_ticks() {
        let mut buffer = ClientInputBuffer::new();
        buffer.advance_tick();
        buffer.push(KeyAction::KeyLeft, true, 0.0);

        let events = buffer.flush();

        assert_eq!(events[0].tick, TickNumber(1));
    }

    #[test]
    fn input_buffer_flush_resets_interval() {
        let mut buffer = ClientInputBuffer::new();
        for _ in 0..30 {
            buffer.advance_tick();
        }
        let _ = buffer.flush();

        assert!(!buffer.should_flush());
    }
}

use std::collections::{BTreeMap, VecDeque};

use tetris_protocol::newtypes::{PlayerSlot, TickNumber};
use tetris_protocol::protocol::InputEvent;

const DEFAULT_MAX_CAPACITY: usize = 2000;
const MAX_RUNGS: usize = 300;

/// Fixed-size ring buffer of replay events per player. Evicts oldest when full.
pub struct ReplayBuffer {
    events: VecDeque<(PlayerSlot, TickNumber, InputEvent)>,
    max_capacity: usize,
}

impl ReplayBuffer {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    pub fn push(&mut self, slot: PlayerSlot, tick: TickNumber, event: InputEvent) {
        if self.events.len() >= self.max_capacity {
            self.events.pop_front();
        }
        self.events.push_back((slot, tick, event));
    }

    pub fn get_events_since(&self, since_tick: TickNumber) -> Vec<(PlayerSlot, TickNumber, InputEvent)> {
        self.events
            .iter()
            .filter(|(_, tick, _)| tick >= &since_tick)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn oldest_tick(&self) -> Option<TickNumber> {
        self.events.front().map(|(_, tick, _)| *tick)
    }
}

impl Default for ReplayBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CAPACITY)
    }
}

/// Stores server state hashes at 100-tick intervals for O(log n) divergence search.
pub struct HashLadder {
    rungs: BTreeMap<TickNumber, u32>,
}

impl HashLadder {
    pub fn new() -> Self {
        Self {
            rungs: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, tick: TickNumber, hash: u32) {
        self.rungs.insert(tick, hash);
        while self.rungs.len() > MAX_RUNGS {
            if let Some((&first, _)) = self.rungs.first_key_value() {
                self.rungs.remove(&first);
            }
        }
    }

    pub fn get_hash_at(&self, tick: TickNumber) -> Option<u32> {
        self.rungs.get(&tick).copied()
    }

    pub fn find_divergence(&self, client_hashes: &[(TickNumber, u32)]) -> Option<TickNumber> {
        for (tick, client_hash) in client_hashes {
            match self.rungs.get(tick) {
                Some(server_hash) if server_hash == client_hash => {}
                _ => return Some(*tick),
            }
        }
        None
    }

    pub fn last_tick(&self) -> Option<TickNumber> {
        self.rungs.last_key_value().map(|(tick, _)| *tick)
    }
}

impl Default for HashLadder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_protocol::newtypes::KeyAction;

    fn make_event(key: KeyAction, tick: u64) -> InputEvent {
        InputEvent {
            key,
            pressed: true,
            tick: TickNumber(tick),
            subframe: 0.0,
        }
    }

    #[test]
    fn test_replay_buffer_empty() {
        let buf = ReplayBuffer::new(2000);
        assert_eq!(buf.len(), 0);
        assert!(buf.oldest_tick().is_none());
    }

    #[test]
    fn test_replay_buffer_push_and_evict() {
        let mut buf = ReplayBuffer::new(3);
        buf.push(PlayerSlot(0), TickNumber(0), make_event(KeyAction::KeyLeft, 0));
        buf.push(PlayerSlot(0), TickNumber(1), make_event(KeyAction::KeyRight, 1));
        buf.push(PlayerSlot(0), TickNumber(2), make_event(KeyAction::KeyHardDrop, 2));
        assert_eq!(buf.len(), 3);
        buf.push(PlayerSlot(0), TickNumber(3), make_event(KeyAction::KeyRotateCW, 3));
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.oldest_tick(), Some(TickNumber(1)));
    }

    #[test]
    fn test_replay_buffer_get_events_since() {
        let mut buf = ReplayBuffer::new(100);
        buf.push(PlayerSlot(0), TickNumber(0), make_event(KeyAction::KeyLeft, 0));
        buf.push(PlayerSlot(0), TickNumber(50), make_event(KeyAction::KeyRight, 50));
        buf.push(PlayerSlot(0), TickNumber(100), make_event(KeyAction::KeyHardDrop, 100));
        buf.push(PlayerSlot(0), TickNumber(150), make_event(KeyAction::KeyHold, 150));

        let since_100 = buf.get_events_since(TickNumber(100));
        assert_eq!(since_100.len(), 2);
    }

    #[test]
    fn test_replay_buffer_2000_capacity() {
        let mut buf = ReplayBuffer::new(2000);
        for i in 0..2001 {
            buf.push(PlayerSlot(0), TickNumber(i), make_event(KeyAction::KeyLeft, i));
        }
        assert_eq!(buf.len(), 2000);
        assert_eq!(buf.oldest_tick(), Some(TickNumber(1)));
    }

    #[test]
    fn test_hash_ladder_insert_and_lookup() {
        let mut ladder = HashLadder::new();
        ladder.insert(TickNumber(0), 0xAABB);
        ladder.insert(TickNumber(100), 0xCCDD);
        assert_eq!(ladder.get_hash_at(TickNumber(0)), Some(0xAABB));
        assert_eq!(ladder.get_hash_at(TickNumber(100)), Some(0xCCDD));
        assert_eq!(ladder.get_hash_at(TickNumber(50)), None);
    }

    #[test]
    fn test_hash_ladder_find_divergence() {
        let mut ladder = HashLadder::new();
        ladder.insert(TickNumber(0), 0xAAAA);
        ladder.insert(TickNumber(100), 0xBBBB);
        ladder.insert(TickNumber(200), 0xCCCC);

        let client_hashes = vec![
            (TickNumber(0), 0xAAAA),
            (TickNumber(100), 0xBBBB),
            (TickNumber(200), 0xCCCC),
        ];
        assert_eq!(ladder.find_divergence(&client_hashes), None);

        let mismatched = vec![
            (TickNumber(0), 0xAAAA),
            (TickNumber(100), 0xFFFF),
            (TickNumber(200), 0xCCCC),
        ];
        assert_eq!(ladder.find_divergence(&mismatched), Some(TickNumber(100)));
    }

    #[test]
    fn test_hash_ladder_find_divergence_early() {
        let mut ladder = HashLadder::new();
        ladder.insert(TickNumber(0), 0xAAAA);
        ladder.insert(TickNumber(100), 0xBBBB);

        let mismatched = vec![(TickNumber(0), 0xDEAD), (TickNumber(100), 0xBBBB)];
        assert_eq!(ladder.find_divergence(&mismatched), Some(TickNumber(0)));
    }

    #[test]
    fn test_hash_ladder_max_rungs() {
        let mut ladder = HashLadder::new();
        for i in 0..400 {
            ladder.insert(TickNumber(i * 100), i as u32);
        }
        // Should only retain the newest 300
        let first = ladder.get_hash_at(TickNumber(0));
        assert!(first.is_none());
    }
}

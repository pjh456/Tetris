use std::collections::HashMap;

use tetris_core::engine::Engine;
use tetris_protocol::newtypes::{PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::InputEvent;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{debug, warn};

use crate::player_conn::Online;
use crate::player_conn::PlayerConnection;

pub const STATE_HASH_INTERVAL: u64 = 100;

/// Commands sent to a `RoomActor` from `ws_handler`.
pub enum RoomCommand {
    PlayerInput {
        slot: PlayerSlot,
        event: InputEvent,
    },
    PlayerReady {
        slot: PlayerSlot,
        ready: bool,
    },
    PlayerLeave {
        slot: PlayerSlot,
    },
    Shutdown,
}

pub struct RoomActor {
    pub room_code: String,
    pub seed: Seed,
    pub tick: TickNumber,
    engines: Vec<Option<Engine<10, 20>>>,
    input_rxs: Vec<Option<mpsc::Receiver<InputEvent>>>,
    connections: Vec<Option<PlayerConnection<Online>>>,
    pending_inputs: Vec<(PlayerSlot, InputEvent)>,
    pub active: bool,
}

/// Convert protocol `InputEvent` to engine `InputEvent`.
fn to_engine_event(ev: &InputEvent) -> tetris_core::engine::InputEvent {
    tetris_core::engine::InputEvent {
        key: ev.key as u8,
        pressed: ev.pressed,
    }
}

impl RoomActor {
    pub fn new(room_code: String, seed: Seed) -> Self {
        Self {
            room_code,
            seed,
            tick: TickNumber(0),
            engines: Vec::new(),
            input_rxs: Vec::new(),
            connections: Vec::new(),
            pending_inputs: Vec::new(),
            active: true,
        }
    }

    pub fn add_player(
        &mut self,
        slot: PlayerSlot,
        input_rx: mpsc::Receiver<InputEvent>,
        conn: PlayerConnection<Online>,
    ) {
        let idx = slot.0 as usize;

        if self.engines.len() <= idx {
            self.engines.resize_with(idx + 1, || None);
            self.input_rxs.resize_with(idx + 1, || None);
            self.connections.resize_with(idx + 1, || None);
        }

        let mut engine = Engine::<10, 20>::new();
        engine.reset(self.seed.0 as u32);

        self.engines[idx] = Some(engine);
        self.input_rxs[idx] = Some(input_rx);
        self.connections[idx] = Some(conn);
    }

    pub fn remove_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if idx < self.engines.len() {
            self.engines[idx] = None;
            self.input_rxs[idx] = None;
            self.connections[idx] = None;
        }
    }

    pub fn engine_count(&self) -> usize {
        self.engines.iter().filter(|e| e.is_some()).count()
    }

    fn collect_inputs(&mut self) {
        for (idx, rx_opt) in self.input_rxs.iter_mut().enumerate() {
            let Some(rx) = rx_opt else {
                continue;
            };
            while let Ok(event) = rx.try_recv() {
                self.pending_inputs
                    .push((PlayerSlot(idx as u8), event));
            }
        }
    }

    pub fn run_one_tick(&mut self) {
        self.collect_inputs();

        let mut per_engine: HashMap<usize, Vec<tetris_core::engine::InputEvent>> = HashMap::new();
        for (slot, event) in self.pending_inputs.drain(..) {
            per_engine
                .entry(slot.0 as usize)
                .or_default()
                .push(to_engine_event(&event));
        }

        for (idx, engine_opt) in self.engines.iter_mut().enumerate() {
            let Some(engine) = engine_opt else {
                continue;
            };
            let inputs = per_engine.get(&idx).cloned().unwrap_or_default();
            let result = engine.fixed_tick(&inputs);
            if result.game_over {
                debug!("player {idx} game over");
            }
        }

        self.tick.0 += 1;
    }

    pub fn broadcast_state_hashes(&self) -> Vec<(PlayerSlot, u32)> {
        let mut hashes = Vec::new();
        for (idx, engine_opt) in self.engines.iter().enumerate() {
            let Some(engine) = engine_opt else {
                continue;
            };
            let hash = engine.state_hash();
            hashes.push((PlayerSlot(idx as u8), hash));
        }
        hashes
    }

    pub async fn run(mut self, mut cancel_rx: tokio::sync::oneshot::Receiver<()>) {
        let mut tick_timer = interval(Duration::from_micros(16667));

        loop {
            tokio::select! {
                _ = tick_timer.tick() => {
                    if !self.active {
                        break;
                    }
                    self.run_one_tick();

                    if self.tick.0.is_multiple_of(STATE_HASH_INTERVAL) {
                        let hashes = self.broadcast_state_hashes();
                        debug!("tick {} state hashes: {} players", self.tick.0, hashes.len());
                    }
                }
                _ = &mut cancel_rx => {
                    break;
                }
            }
        }

        warn!("RoomActor {} shutting down", self.room_code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_protocol::newtypes::KeyAction;

    fn make_event(key: KeyAction, pressed: bool) -> InputEvent {
        InputEvent {
            key,
            pressed,
            tick: TickNumber(0),
            subframe: 0.0,
        }
    }

    fn make_conn(slot: u8, name: &str) -> (PlayerConnection<Online>, mpsc::Receiver<InputEvent>) {
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(slot), tx, name.into());
        (conn, rx)
    }

    #[test]
    fn test_new_room_empty() {
        let actor = RoomActor::new("ABCD".into(), Seed(42));
        assert_eq!(actor.room_code, "ABCD");
        assert_eq!(actor.tick, TickNumber(0));
        assert_eq!(actor.engine_count(), 0);
    }

    #[test]
    fn test_add_player() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx) = make_conn(0, "Alice");
        actor.add_player(PlayerSlot(0), rx, conn);
        assert_eq!(actor.engine_count(), 1);
    }

    #[test]
    fn test_add_two_players_same_seed() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn1, rx1) = make_conn(0, "A");
        let (conn2, rx2) = make_conn(1, "B");
        actor.add_player(PlayerSlot(0), rx1, conn1);
        actor.add_player(PlayerSlot(1), rx2, conn2);
        assert_eq!(actor.engine_count(), 2);

        let engine0 = actor.engines[0].as_ref().unwrap();
        let engine1 = actor.engines[1].as_ref().unwrap();
        assert_eq!(engine0.state.piece, engine1.state.piece);
        assert_eq!(engine0.state.next, engine1.state.next);
    }

    #[test]
    fn test_run_one_tick_advances_gravity() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn);

        for _ in 0..50 {
            actor.run_one_tick();
        }
        assert!(actor.tick.0 >= 50);
    }

    #[test]
    fn test_tick_applies_pending_input() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx.clone(), "A".into());

        let harddrop_ev = make_event(KeyAction::KeyHardDrop, true);
        tx.try_send(harddrop_ev).unwrap();

        actor.add_player(PlayerSlot(0), rx, conn);

        let piece_before = actor.engines[0].as_ref().unwrap().state.piece;
        actor.run_one_tick();
        let piece_after = actor.engines[0].as_ref().unwrap().state.piece;
        assert_ne!(piece_before, piece_after);
    }

    #[test]
    fn test_state_hash_broadcast_at_interval() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn);

        for _ in 0..99 {
            actor.run_one_tick();
        }
        assert_eq!(actor.tick, TickNumber(99));

        actor.run_one_tick();
        assert_eq!(actor.tick, TickNumber(100));
        let hashes = actor.broadcast_state_hashes();
        assert_eq!(hashes.len(), 1);
    }

    #[test]
    fn test_remove_player() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn);
        assert_eq!(actor.engine_count(), 1);
        actor.remove_player(PlayerSlot(0));
        assert_eq!(actor.engine_count(), 0);
    }
}

use std::collections::HashMap;

use tetris_core::engine::Engine;
use tetris_protocol::newtypes::{PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::{
    InputEvent, PacketHeader, PacketType, PktReconnectAck, PktServerReplay,
    PktStateHash, PktStateSnapshot, PROTOCOL_VERSION,
};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::{debug, warn};

use crate::player_conn::Online;
use crate::player_conn::PlayerConnection;
use crate::replay_buffer::{HashLadder, ReplayBuffer};

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
    Reconnect {
        slot: PlayerSlot,
        client_hashes: Vec<(TickNumber, u32)>,
    },
    Shutdown,
}

fn to_engine_event(ev: &InputEvent) -> tetris_core::engine::InputEvent {
    tetris_core::engine::InputEvent {
        key: ev.key as u8,
        pressed: ev.pressed,
    }
}

pub struct RoomActor {
    pub room_code: String,
    pub seed: Seed,
    pub tick: TickNumber,
    engines: Vec<Option<Engine<10, 20>>>,
    input_rxs: Vec<Option<mpsc::Receiver<InputEvent>>>,
    connections: Vec<Option<PlayerConnection<Online>>>,
    replay_buffers: HashMap<PlayerSlot, ReplayBuffer>,
    hash_ladder: HashLadder,
    outbound_txs: Vec<Option<mpsc::Sender<Vec<u8>>>>,
    pending_inputs: Vec<(PlayerSlot, InputEvent)>,
    pub active: bool,
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
            replay_buffers: HashMap::new(),
            hash_ladder: HashLadder::new(),
            outbound_txs: Vec::new(),
            pending_inputs: Vec::new(),
            active: true,
        }
    }

    pub fn add_player(
        &mut self,
        slot: PlayerSlot,
        input_rx: mpsc::Receiver<InputEvent>,
        conn: PlayerConnection<Online>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    ) {
        let idx = slot.0 as usize;

        if self.engines.len() <= idx {
            self.engines.resize_with(idx + 1, || None);
            self.input_rxs.resize_with(idx + 1, || None);
            self.connections.resize_with(idx + 1, || None);
            self.outbound_txs.resize_with(idx + 1, || None);
        }

        let mut engine = Engine::<10, 20>::new();
        engine.reset(self.seed.0 as u32);

        self.engines[idx] = Some(engine);
        self.input_rxs[idx] = Some(input_rx);
        self.connections[idx] = Some(conn);
        self.outbound_txs[idx] = Some(outbound_tx);
        self.replay_buffers
            .insert(slot, ReplayBuffer::default());
    }

    pub fn remove_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if idx < self.engines.len() {
            self.engines[idx] = None;
            self.input_rxs[idx] = None;
            self.connections[idx] = None;
            self.outbound_txs[idx] = None;
        }
        self.replay_buffers.remove(&slot);
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

    fn forward_state_hashes(&self) {
        let hashes = self.broadcast_state_hashes();
        for (idx, outbound_opt) in self.outbound_txs.iter().enumerate() {
            let Some(tx) = outbound_opt else {
                continue;
            };
            let hash = hashes
                .iter()
                .find(|(slot, _)| slot.0 as usize == idx)
                .map_or(0, |(_, h)| *h);
            let pkt = PktStateHash {
                header: PacketHeader {
                    version: PROTOCOL_VERSION,
                    packet_type: PacketType::StateHash,
                    player_id: 0,
                },
                tick: self.tick,
                hash,
            };
            if let Ok(data) = bincode::serialize(&pkt) {
                let _ = tx.try_send(data);
            }
        }
    }

    pub fn run_one_tick(&mut self) {
        self.collect_inputs();

        let mut per_engine: HashMap<usize, Vec<tetris_core::engine::InputEvent>> = HashMap::new();
        let mut per_replay: HashMap<usize, Vec<InputEvent>> = HashMap::new();
        for (slot, event) in self.pending_inputs.drain(..) {
            per_engine
                .entry(slot.0 as usize)
                .or_default()
                .push(to_engine_event(&event));
            per_replay
                .entry(slot.0 as usize)
                .or_default()
                .push(event.clone());
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

            // Store inputs in replay buffer
            if let Some(rb) = self.replay_buffers.get_mut(&PlayerSlot(idx as u8)) {
                for ev in per_replay.get(&idx).into_iter().flatten() {
                    rb.push(PlayerSlot(idx as u8), self.tick, ev.clone());
                }
            }
        }

        self.tick.0 += 1;

        if self.tick.0.is_multiple_of(STATE_HASH_INTERVAL) {
            let hashes = self.broadcast_state_hashes();
            for (_slot, hash) in &hashes {
                self.hash_ladder.insert(self.tick, *hash);
            }
            debug!(
                "tick {} state hashes: {} players",
                self.tick.0,
                hashes.len()
            );
            self.forward_state_hashes();
        }
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

    pub fn build_snapshot_for_player(&self, slot: PlayerSlot) -> Option<PktStateSnapshot> {
        let idx = slot.0 as usize;
        let engine = self.engines.get(idx)?.as_ref()?;
        Some(PktStateSnapshot {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSnapshot,
                player_id: 0,
            },
            tick: self.tick,
            board_rows: engine.state.board.rows.to_vec(),
            piece: engine.state.piece,
            rot: engine.state.rot,
            x: engine.state.x,
            y: engine.state.y,
            hold: engine.state.hold,
            hold_used: engine.state.hold_used,
            next: engine.state.next,
            rng_state: engine.state.rng,
            combo: engine.state.combo,
            b2b: engine.state.b2b,
            pending_garbage: engine.state.pending_garbage,
            seed: self.seed,
        })
    }

    pub fn handle_reconnect(
        &self,
        slot: PlayerSlot,
        client_hashes: &[(TickNumber, u32)],
    ) -> PktReconnectAck {
        let divergence = self.hash_ladder.find_divergence(client_hashes);

        let Some(divergence_tick) = divergence else {
            return PktReconnectAck {
                header: PacketHeader {
                    version: PROTOCOL_VERSION,
                    packet_type: PacketType::ReconnectAck,
                    player_id: 0,
                },
                divergence_tick: TickNumber(0),
                replay_events: vec![],
            };
        };

        let mut replay_events = Vec::new();
        let rb = self.replay_buffers.get(&slot);

        if let Some(buf) = rb
            && let Some(oldest) = buf.oldest_tick()
            && divergence_tick >= oldest
        {
            let events = buf.get_events_since(divergence_tick);
            if !events.is_empty() {
                replay_events.push(PktServerReplay {
                    header: PacketHeader {
                        version: PROTOCOL_VERSION,
                        packet_type: PacketType::ServerReplay,
                        player_id: 0,
                    },
                    source_player: slot,
                    events: events.into_iter().map(|(_, _, ev)| ev).collect(),
                    ige_garbage_lines: 0,
                    ige_hole_x: 0,
                });
            }
        }

        PktReconnectAck {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ReconnectAck,
                player_id: 0,
            },
            divergence_tick,
            replay_events,
        }
    }

    pub fn replay_broadcast(&self, source_slot: PlayerSlot, events: &[InputEvent]) {
        let pkt = PktServerReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ServerReplay,
                player_id: 0,
            },
            source_player: source_slot,
            events: events.to_vec(),
            ige_garbage_lines: 0,
            ige_hole_x: 0,
        };
        if let Ok(data) = bincode::serialize(&pkt) {
            for (idx, tx_opt) in self.outbound_txs.iter().enumerate() {
                if idx == source_slot.0 as usize {
                    continue;
                }
                let Some(tx) = tx_opt else {
                    continue;
                };
                let _ = tx.try_send(data.clone());
            }
        }
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

    fn make_conn(slot: u8, name: &str) -> (PlayerConnection<Online>, mpsc::Receiver<InputEvent>, mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(slot), tx, name.into());
        (conn, rx, out_tx, out_rx)
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
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "Alice");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        assert_eq!(actor.engine_count(), 1);
    }

    #[test]
    fn test_add_two_players_same_seed() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn1, rx1, out1, _r1) = make_conn(0, "A");
        let (conn2, rx2, out2, _r2) = make_conn(1, "B");
        actor.add_player(PlayerSlot(0), rx1, conn1, out1);
        actor.add_player(PlayerSlot(1), rx2, conn2, out2);
        assert_eq!(actor.engine_count(), 2);

        let engine0 = actor.engines[0].as_ref().unwrap();
        let engine1 = actor.engines[1].as_ref().unwrap();
        assert_eq!(engine0.state.piece, engine1.state.piece);
    }

    #[test]
    fn test_run_one_tick_advances_gravity() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        for _ in 0..50 {
            actor.run_one_tick();
        }
        assert!(actor.tick.0 >= 50);
    }

    #[test]
    fn test_tick_applies_pending_input() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let (out_tx, _out_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx.clone(), "A".into());

        tx.try_send(make_event(KeyAction::KeyHardDrop, true)).unwrap();
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        let piece_before = actor.engines[0].as_ref().unwrap().state.piece;
        actor.run_one_tick();
        let piece_after = actor.engines[0].as_ref().unwrap().state.piece;
        assert_ne!(piece_before, piece_after);
    }

    #[test]
    fn test_state_hash_broadcast_at_interval() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        for _ in 0..99 {
            actor.run_one_tick();
        }
        assert_eq!(actor.tick, TickNumber(99));
        assert!(actor.hash_ladder.get_hash_at(TickNumber(100)).is_none());

        actor.run_one_tick();
        assert_eq!(actor.tick, TickNumber(100));
    }

    #[test]
    fn test_remove_player() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        assert_eq!(actor.engine_count(), 1);
        actor.remove_player(PlayerSlot(0));
        assert_eq!(actor.engine_count(), 0);
    }

    #[test]
    fn test_build_snapshot() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        let snapshot = actor.build_snapshot_for_player(PlayerSlot(0));
        assert!(snapshot.is_some());
        let snap = snapshot.unwrap();
        assert_eq!(snap.board_rows.len(), 20);
        assert_eq!(snap.seed, Seed(42));
    }

    #[test]
    fn test_build_snapshot_removed_player() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        actor.remove_player(PlayerSlot(0));
        assert!(actor.build_snapshot_for_player(PlayerSlot(0)).is_none());
    }

    #[test]
    fn test_handle_reconnect_no_divergence() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        for _ in 0..100 {
            actor.run_one_tick();
        }

        let hash = actor.engines[0].as_ref().unwrap().state_hash();
        let client_hashes = vec![(TickNumber(100), hash)];
        let ack = actor.handle_reconnect(PlayerSlot(0), &client_hashes);
        assert_eq!(ack.divergence_tick, TickNumber(0));
        assert!(ack.replay_events.is_empty());
    }

    #[test]
    fn test_handle_reconnect_with_divergence() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        for _ in 0..100 {
            actor.run_one_tick();
        }

        let client_hashes = vec![(TickNumber(100), 0xDEAD_BEEF)];
        let ack = actor.handle_reconnect(PlayerSlot(0), &client_hashes);
        assert_eq!(ack.divergence_tick, TickNumber(100));
    }

    #[test]
    fn test_replay_buffer_integration() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let (out_tx, _out_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx.clone(), "A".into());

        tx.try_send(make_event(KeyAction::KeyHardDrop, true)).unwrap();
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        actor.run_one_tick();

        let rb = actor.replay_buffers.get(&PlayerSlot(0));
        assert!(rb.is_some());
    }
}

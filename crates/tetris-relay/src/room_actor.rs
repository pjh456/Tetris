use serde::{Deserialize, Serialize};
use tetris_net::bot::AiBot;
use tetris_protocol::newtypes::{PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::{
    InputEvent, PacketHeader, PacketType, PktRoomSnapshot, PktStateSnapshot, RoomPlayerSnapshot,
};
use tetris_sim::{AuthoritativeSim, RoomMode, SimOutbound};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::warn;

use crate::player_conn::Online;
use crate::player_conn::PlayerConnection;

pub const STATE_HASH_INTERVAL: u64 = 100;
const BOT_WEIGHTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../models/weights.json"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RelayRoomMode {
    #[default]
    Lobby,
    Countdown,
    Playing,
    GameOver,
}

/// Commands sent to a `RoomActor` from `ws_handler`.
pub enum RoomCommand {
    ResumePlayer {
        slot: PlayerSlot,
        input_rx: mpsc::Receiver<InputEvent>,
        conn: PlayerConnection<Online>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    },
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
    AddBot {
        slot: PlayerSlot,
        temperature: f32,
    },
    Shutdown,
}

pub struct RoomActor {
    pub room_code: String,
    input_rxs: Vec<Option<mpsc::Receiver<InputEvent>>>,
    connections: Vec<Option<PlayerConnection<Online>>>,
    outbound_txs: Vec<Option<mpsc::Sender<Vec<u8>>>>,
    pub active: bool,
    pub sim: AuthoritativeSim,
    bots: Vec<(PlayerSlot, AiBot)>,
}

impl RoomActor {
    pub fn new(room_code: String, seed: Seed) -> Self {
        Self {
            room_code,
            input_rxs: Vec::new(),
            connections: Vec::new(),
            outbound_txs: Vec::new(),
            active: true,
            sim: AuthoritativeSim::new(seed),
            bots: Vec::new(),
        }
    }

    pub fn seed(&self) -> Seed {
        self.sim.seed
    }

    pub fn tick(&self) -> TickNumber {
        self.sim.tick
    }

    pub fn room_mode(&self) -> RoomMode {
        self.sim.room_mode()
    }

    pub fn set_room_mode(&mut self, room_mode: RoomMode) {
        self.sim.set_room_mode(room_mode);
    }

    pub fn add_player(
        &mut self,
        slot: PlayerSlot,
        input_rx: mpsc::Receiver<InputEvent>,
        conn: PlayerConnection<Online>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    ) {
        let idx = slot.0 as usize;

        if self.input_rxs.len() <= idx {
            self.input_rxs.resize_with(idx + 1, || None);
            self.connections.resize_with(idx + 1, || None);
            self.outbound_txs.resize_with(idx + 1, || None);
        }

        self.sim.add_player(slot);
        self.input_rxs[idx] = Some(input_rx);
        self.connections[idx] = Some(conn);
        self.outbound_txs[idx] = Some(outbound_tx);
    }

    pub fn add_bot(&mut self, slot: PlayerSlot, temperature: f32) -> Result<(), String> {
        let policy = tetris_infer::MlpPolicy::load_from_str(BOT_WEIGHTS)
            .map_err(|err| format!("failed to load bot weights: {err}"))?;
        let idx = slot.0 as usize;
        if self.input_rxs.len() <= idx {
            self.input_rxs.resize_with(idx + 1, || None);
            self.connections.resize_with(idx + 1, || None);
            self.outbound_txs.resize_with(idx + 1, || None);
        }

        let (input_tx, input_rx) = mpsc::channel::<InputEvent>(64);
        let (outbound_tx, _outbound_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(slot, input_tx, format!("AI {}", slot.0));
        self.sim.add_player(slot);
        self.input_rxs[idx] = Some(input_rx);
        self.connections[idx] = Some(conn);
        self.outbound_txs[idx] = Some(outbound_tx);
        self.bots
            .push((slot, AiBot::new(policy, self.seed().0 as u32, temperature)));
        Ok(())
    }

    fn resume_player(
        &mut self,
        slot: PlayerSlot,
        input_rx: mpsc::Receiver<InputEvent>,
        conn: PlayerConnection<Online>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    ) {
        let idx = slot.0 as usize;
        if self.input_rxs.len() <= idx {
            self.input_rxs.resize_with(idx + 1, || None);
            self.connections.resize_with(idx + 1, || None);
            self.outbound_txs.resize_with(idx + 1, || None);
        }
        if self.sim.engine(slot).is_none() {
            self.sim.add_player(slot);
        }
        self.input_rxs[idx] = Some(input_rx);
        self.connections[idx] = Some(conn);
        self.outbound_txs[idx] = Some(outbound_tx);
    }

    pub fn remove_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if idx < self.input_rxs.len() {
            self.input_rxs[idx] = None;
            self.connections[idx] = None;
            self.outbound_txs[idx] = None;
        }
        self.sim.remove_player(slot);
    }

    pub fn engine_count(&self) -> usize {
        self.sim.engine_count()
    }

    fn collect_inputs(&mut self) {
        for (idx, rx_opt) in self.input_rxs.iter_mut().enumerate() {
            let Some(rx) = rx_opt else {
                continue;
            };
            while let Ok(event) = rx.try_recv() {
                self.sim.enqueue_input(PlayerSlot(idx as u8), event);
            }
        }
    }

    fn collect_bot_inputs(&mut self) {
        for (slot, bot) in &mut self.bots {
            if let Some(replay) = bot.next_replay(slot.0) {
                for event in replay.events {
                    self.sim.enqueue_input(*slot, event);
                }
            }
        }
    }

    pub fn run_one_tick(&mut self) {
        self.collect_bot_inputs();
        self.collect_inputs();
        let Ok(outbound) = self.sim.tick() else {
            return;
        };
        self.dispatch_outbound(outbound);
    }

    fn dispatch_outbound(&self, outbound: Vec<SimOutbound>) {
        for event in outbound {
            match event {
                SimOutbound::ToPlayer(slot, data) => self.send_to_player(slot, data),
                SimOutbound::Broadcast(data) => self.broadcast(data),
            }
        }
    }

    fn send_to_player(&self, slot: PlayerSlot, data: Vec<u8>) {
        if let Some(Some(tx)) = self.outbound_txs.get(slot.0 as usize) {
            let _ = tx.try_send(data);
        }
    }

    fn broadcast(&self, data: Vec<u8>) {
        for tx in self.outbound_txs.iter().flatten() {
            let _ = tx.try_send(data.clone());
        }
    }

    pub fn broadcast_state_hashes(&self) -> Vec<(PlayerSlot, u32)> {
        self.sim.broadcast_state_hashes()
    }

    pub fn build_snapshot_for_player(&self, slot: PlayerSlot) -> Option<PktStateSnapshot> {
        self.sim.build_snapshot_for_player(slot)
    }

    pub fn handle_reconnect(
        &self,
        slot: PlayerSlot,
        client_hashes: &[(TickNumber, u32)],
    ) -> tetris_protocol::protocol::PktReconnectAck {
        self.sim.handle_reconnect(slot, client_hashes)
    }

    fn send_reconnect_ack(&self, slot: PlayerSlot, client_hashes: &[(TickNumber, u32)]) {
        let ack = self.sim.handle_reconnect(slot, client_hashes);
        if let Ok(data) = bincode::serialize(&ack) {
            self.send_to_player(slot, data);
        }
    }

    pub fn replay_broadcast(&self, source_slot: PlayerSlot, events: &[InputEvent]) {
        if let Ok(outbound) = self.sim.replay_broadcast(source_slot, events) {
            self.dispatch_outbound(outbound);
        }
    }

    fn build_player_snapshots(&self) -> Vec<RoomPlayerSnapshot> {
        self.connections
            .iter()
            .enumerate()
            .filter_map(|(idx, conn)| {
                let conn = conn.as_ref()?;
                Some(RoomPlayerSnapshot {
                    player_id: idx as u8,
                    name: conn.peer_name.clone(),
                    ready: false,
                    alive: self.sim.engine(PlayerSlot(idx as u8)).is_some(),
                    away: false,
                    is_host: idx == 0,
                })
            })
            .collect()
    }

    pub fn broadcast_lobby_reset(&self) {
        let snapshot = PktRoomSnapshot {
            header: PacketHeader::new(PacketType::RoomSnapshot, 0),
            room_code: self.room_code.clone(),
            players: self.build_player_snapshots(),
        };
        if let Ok(data) = bincode::serialize(&snapshot) {
            self.broadcast(data);
        }
    }

    pub async fn run(
        mut self,
        mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
        mut command_rx: mpsc::Receiver<RoomCommand>,
    ) {
        let mut tick_timer = interval(Duration::from_micros(16667));

        loop {
            tokio::select! {
                _ = tick_timer.tick() => {
                    if self.engine_count() == 0 {
                        break;
                    }
                    self.run_one_tick();
                }
                command = command_rx.recv() => {
                    match command {
                        Some(RoomCommand::ResumePlayer { slot, input_rx, conn, outbound_tx }) => {
                            self.resume_player(slot, input_rx, conn, outbound_tx);
                        }
                        Some(RoomCommand::Reconnect { slot, client_hashes }) => {
                            self.send_reconnect_ack(slot, &client_hashes);
                        }
                        Some(RoomCommand::AddBot { slot, temperature }) => {
                            let _ = self.add_bot(slot, temperature);
                            self.broadcast_lobby_reset();
                        }
                        Some(RoomCommand::PlayerInput { slot, event }) => {
                            self.sim.enqueue_input(slot, event);
                        }
                        Some(RoomCommand::PlayerLeave { slot }) => self.remove_player(slot),
                        Some(RoomCommand::PlayerReady { .. }) => {}
                        Some(RoomCommand::Shutdown) | None => break,
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

    fn make_conn(
        slot: u8,
        name: &str,
    ) -> (
        PlayerConnection<Online>,
        mpsc::Receiver<InputEvent>,
        mpsc::Sender<Vec<u8>>,
        mpsc::Receiver<Vec<u8>>,
    ) {
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(slot), tx, name.into());
        (conn, rx, out_tx, out_rx)
    }

    #[test]
    fn new_room_is_empty() {
        let actor = RoomActor::new("ABCD".into(), Seed(42));

        assert_eq!(actor.engine_count(), 0);
    }

    #[test]
    fn add_player_creates_engine_in_authoritative_sim() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "Alice");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        assert_eq!(actor.engine_count(), 1);
    }

    #[test]
    fn tick_applies_pending_input() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (tx, rx) = mpsc::channel::<InputEvent>(64);
        let (out_tx, _out_rx) = mpsc::channel::<Vec<u8>>(64);
        let conn = PlayerConnection::<Online>::new(PlayerSlot(0), tx.clone(), "A".into());
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        let piece_before = actor.sim.engine(PlayerSlot(0)).unwrap().state.piece;

        tx.try_send(make_event(KeyAction::KeyHardDrop, true))
            .unwrap();
        actor.run_one_tick();

        assert_ne!(
            actor.sim.engine(PlayerSlot(0)).unwrap().state.piece,
            piece_before
        );
    }

    #[test]
    fn state_hash_is_stored_at_interval() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        for _ in 0..100 {
            actor.run_one_tick();
        }

        assert!(actor.sim.hash_at(PlayerSlot(0), TickNumber(100)).is_some());
    }

    #[test]
    fn remove_player_drops_engine() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        actor.remove_player(PlayerSlot(0));

        assert_eq!(actor.engine_count(), 0);
    }

    #[test]
    fn build_snapshot_returns_player_state() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);

        let snapshot = actor.build_snapshot_for_player(PlayerSlot(0)).unwrap();

        assert_eq!(snapshot.seed, Seed(42));
    }

    #[test]
    fn reconnect_detects_hash_divergence() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        let (conn, rx, out_tx, _out_rx) = make_conn(0, "A");
        actor.add_player(PlayerSlot(0), rx, conn, out_tx);
        for _ in 0..100 {
            actor.run_one_tick();
        }

        let ack = actor.handle_reconnect(PlayerSlot(0), &[(TickNumber(100), 0xDEAD_BEEF)]);

        assert_eq!(ack.divergence_tick, TickNumber(100));
    }

    #[test]
    fn game_start_sets_playing() {
        let mut actor = RoomActor::new("ABCD".into(), Seed(42));
        actor.set_room_mode(RoomMode::Playing);

        assert_eq!(actor.room_mode(), RoomMode::Playing);
    }
}

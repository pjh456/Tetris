use std::collections::HashMap;

use tetris_core::engine::Engine;
use tetris_protocol::newtypes::{PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::{
    InputEvent, PROTOCOL_VERSION, PacketHeader, PacketType, PktGameOver, PktIncomingGarbage,
    PktPlayerStateSync, PktReconnectAck, PktServerReplay, PktStateHash, PktStateSnapshot,
};

use crate::replay::{HashLadder, ReplayBuffer};
use crate::snapshot::build_snapshot;
use crate::transport::SimOutbound;

pub const DEFAULT_STATE_HASH_INTERVAL: u64 = 100;
type OutgoingAttack = (PlayerSlot, u8, u8);
type IncomingGarbage = (PlayerSlot, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimConfig {
    pub state_hash_interval: u64,
    pub garbage_delay_ticks: u16,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            state_hash_interval: DEFAULT_STATE_HASH_INTERVAL,
            garbage_delay_ticks: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomMode {
    #[default]
    Lobby,
    Countdown,
    Playing,
    GameOver,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SimError {
    #[error("packet serialization failed")]
    PacketSerialize,
}

pub struct AuthoritativeSim {
    pub seed: Seed,
    pub tick: TickNumber,
    config: SimConfig,
    engines: Vec<Option<Engine<10, 20>>>,
    replay_buffers: HashMap<PlayerSlot, ReplayBuffer>,
    hash_ladder: HashLadder,
    pending_inputs: Vec<(PlayerSlot, InputEvent)>,
    player_alive: Vec<bool>,
    player_spectating: Vec<Option<PlayerSlot>>,
    room_mode: RoomMode,
    game_over_countdown: u8,
}

impl AuthoritativeSim {
    pub fn new(seed: Seed) -> Self {
        Self::with_config(seed, SimConfig::default())
    }

    pub fn with_config(seed: Seed, config: SimConfig) -> Self {
        Self {
            seed,
            tick: TickNumber(0),
            config,
            engines: Vec::new(),
            replay_buffers: HashMap::new(),
            hash_ladder: HashLadder::new(),
            pending_inputs: Vec::new(),
            player_alive: Vec::new(),
            player_spectating: Vec::new(),
            room_mode: RoomMode::Lobby,
            game_over_countdown: 0,
        }
    }

    pub fn add_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if self.engines.len() <= idx {
            self.engines.resize_with(idx + 1, || None);
            self.player_alive.resize_with(idx + 1, || true);
            self.player_spectating.resize_with(idx + 1, || None);
        }

        let mut engine = Engine::<10, 20>::new();
        engine.reset(self.seed.0 as u32);
        self.engines[idx] = Some(engine);
        self.player_alive[idx] = true;
        self.player_spectating[idx] = None;
        self.replay_buffers.insert(slot, ReplayBuffer::default());
    }

    pub fn remove_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if idx < self.engines.len() {
            self.engines[idx] = None;
            self.player_alive[idx] = false;
            self.player_spectating[idx] = None;
        }
        self.replay_buffers.remove(&slot);
    }

    pub fn engine_count(&self) -> usize {
        self.engines
            .iter()
            .filter(|engine| engine.is_some())
            .count()
    }

    pub fn enqueue_input(&mut self, slot: PlayerSlot, event: InputEvent) {
        self.pending_inputs.push((slot, event));
    }

    pub fn set_room_mode(&mut self, room_mode: RoomMode) {
        self.room_mode = room_mode;
    }

    pub fn room_mode(&self) -> RoomMode {
        self.room_mode
    }

    pub fn engine(&self, slot: PlayerSlot) -> Option<&Engine<10, 20>> {
        self.engines.get(slot.0 as usize)?.as_ref()
    }

    pub fn engine_mut(&mut self, slot: PlayerSlot) -> Option<&mut Engine<10, 20>> {
        self.engines.get_mut(slot.0 as usize)?.as_mut()
    }

    pub fn replay_buffer(&self, slot: PlayerSlot) -> Option<&ReplayBuffer> {
        self.replay_buffers.get(&slot)
    }

    pub fn hash_at(&self, tick: TickNumber) -> Option<u32> {
        self.hash_ladder.get_hash_at(tick)
    }

    pub fn build_snapshot_for_player(&self, slot: PlayerSlot) -> Option<PktStateSnapshot> {
        let engine = self.engine(slot)?;
        Some(build_snapshot(engine, self.tick, self.seed))
    }

    pub fn broadcast_state_hashes(&self) -> Vec<(PlayerSlot, u32)> {
        self.engines
            .iter()
            .enumerate()
            .filter_map(|(idx, engine)| {
                engine
                    .as_ref()
                    .map(|engine| (PlayerSlot(idx as u8), engine.state_hash()))
            })
            .collect()
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
                snapshot: None,
            };
        };

        let mut replay_events = Vec::new();
        if let Some(buf) = self.replay_buffers.get(&slot)
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

        let snapshot = if replay_events.is_empty() {
            self.build_snapshot_for_player(slot)
        } else {
            None
        };

        PktReconnectAck {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ReconnectAck,
                player_id: 0,
            },
            divergence_tick,
            replay_events,
            snapshot,
        }
    }

    pub fn tick(&mut self) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();
        let per_replay = self.drain_inputs_by_player();
        outbound.extend(self.server_replay_outbound(&per_replay)?);
        let (outgoing_attacks, mut incoming_notify) = self.tick_engines(&per_replay);

        outbound.extend(self.update_lifecycle()?);
        self.route_attacks(&outgoing_attacks, &mut incoming_notify);
        outbound.extend(Self::incoming_garbage_outbound(&incoming_notify)?);

        self.tick.0 += 1;

        if self.tick.0.is_multiple_of(self.config.state_hash_interval) {
            let hashes = self.broadcast_state_hashes();
            for (_slot, hash) in &hashes {
                self.hash_ladder.insert(self.tick, *hash);
            }
            outbound.extend(self.state_hash_outbound(&hashes)?);
            outbound.extend(self.state_snapshot_outbound()?);
        }

        Ok(outbound)
    }

    pub fn replay_broadcast(
        &self,
        source_slot: PlayerSlot,
        events: &[InputEvent],
    ) -> Result<Vec<SimOutbound>, SimError> {
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
        let data = serialize_packet(&pkt)?;
        Ok(self
            .engines
            .iter()
            .enumerate()
            .filter(|(idx, engine)| engine.is_some() && *idx != source_slot.0 as usize)
            .map(|(idx, _)| SimOutbound::ToPlayer(PlayerSlot(idx as u8), data.clone()))
            .collect())
    }

    fn drain_inputs_by_player(&mut self) -> HashMap<usize, Vec<InputEvent>> {
        let mut per_replay: HashMap<usize, Vec<InputEvent>> = HashMap::new();
        for (slot, event) in self.pending_inputs.drain(..) {
            let idx = slot.0 as usize;
            per_replay.entry(idx).or_default().push(event.clone());
            if let Some(rb) = self.replay_buffers.get_mut(&slot) {
                rb.push(slot, self.tick, event);
            }
        }
        per_replay
    }

    fn tick_engines(
        &mut self,
        per_replay: &HashMap<usize, Vec<InputEvent>>,
    ) -> (Vec<OutgoingAttack>, Vec<IncomingGarbage>) {
        let mut outgoing_attacks = Vec::new();
        let mut incoming_notify = Vec::new();

        for (idx, engine_opt) in self.engines.iter_mut().enumerate() {
            let Some(engine) = engine_opt else {
                continue;
            };
            let inputs = per_replay
                .get(&idx)
                .into_iter()
                .flatten()
                .map(to_engine_event)
                .collect::<Vec<_>>();
            let result = engine.fixed_tick(&inputs);
            let slot = PlayerSlot(idx as u8);

            if let Some(attack) = &result.attack
                && attack.damage > 0
            {
                outgoing_attacks.push((slot, attack.damage as u8, attack.hole_x));
            }

            if result.incoming_garbage_lines > 0 {
                incoming_notify.push((slot, result.incoming_garbage_lines));
            }
        }

        (outgoing_attacks, incoming_notify)
    }

    fn server_replay_outbound(
        &self,
        per_replay: &HashMap<usize, Vec<InputEvent>>,
    ) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();
        for (source_idx, events) in per_replay {
            if events.is_empty() {
                continue;
            }
            let source_slot = PlayerSlot(*source_idx as u8);
            let pkt = PktServerReplay {
                header: PacketHeader {
                    version: PROTOCOL_VERSION,
                    packet_type: PacketType::ServerReplay,
                    player_id: 0,
                },
                source_player: source_slot,
                events: events.clone(),
                ige_garbage_lines: 0,
                ige_hole_x: 0,
            };
            let data = serialize_packet(&pkt)?;
            outbound.extend(self.engines.iter().enumerate().filter_map(|(idx, engine)| {
                if engine.is_some() && idx != *source_idx {
                    Some(SimOutbound::ToPlayer(PlayerSlot(idx as u8), data.clone()))
                } else {
                    None
                }
            }));
        }
        Ok(outbound)
    }

    fn update_lifecycle(&mut self) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();

        if self.room_mode == RoomMode::Playing {
            let mut newly_dead = Vec::new();
            for (idx, engine_opt) in self.engines.iter().enumerate() {
                if let Some(engine) = engine_opt
                    && engine.game_over
                    && self.player_alive.get(idx).copied().unwrap_or(true)
                {
                    newly_dead.push(PlayerSlot(idx as u8));
                }
            }

            for slot in &newly_dead {
                let idx = slot.0 as usize;
                self.player_alive[idx] = false;
                self.player_spectating[idx] = self
                    .engines
                    .iter()
                    .enumerate()
                    .find(|(i, engine)| {
                        engine.as_ref().is_some_and(|eng| !eng.game_over) && *i != idx
                    })
                    .map(|(i, _)| PlayerSlot(i as u8));
                outbound.push(SimOutbound::Broadcast(self.player_state_packet(*slot)?));
            }

            let alive_count = self.player_alive.iter().filter(|&&alive| alive).count();
            if alive_count <= 1 && !newly_dead.is_empty() {
                let winner_id = self
                    .player_alive
                    .iter()
                    .enumerate()
                    .find(|(_, alive)| **alive)
                    .map_or(0, |(idx, _)| idx as u8);
                self.room_mode = RoomMode::GameOver;
                self.game_over_countdown = 180;
                outbound.push(SimOutbound::Broadcast(Self::game_over_packet(winner_id)?));
            }
        }

        if self.room_mode == RoomMode::GameOver {
            if self.game_over_countdown > 0 {
                self.game_over_countdown -= 1;
            }
            if self.game_over_countdown == 0 {
                self.room_mode = RoomMode::Lobby;
                self.reset_to_lobby();
            }
        }

        Ok(outbound)
    }

    fn route_attacks(
        &mut self,
        outgoing_attacks: &[OutgoingAttack],
        incoming_notify: &mut Vec<IncomingGarbage>,
    ) {
        let garbage_delay_ticks = self.config.garbage_delay_ticks;
        for (source_slot, damage, hole_x) in outgoing_attacks {
            if let Some(target) = self.route_attack_target(*source_slot) {
                if let Some(engine) = self.engine_mut(target) {
                    engine.add_pending_garbage(*damage, *hole_x, garbage_delay_ticks);
                }
                incoming_notify.push((target, *damage));
            }
        }
    }

    fn route_attack_target(&self, source: PlayerSlot) -> Option<PlayerSlot> {
        let alive: Vec<usize> = self
            .engines
            .iter()
            .enumerate()
            .filter(|(idx, engine)| {
                engine.as_ref().is_some_and(|eng| !eng.game_over) && *idx != source.0 as usize
            })
            .map(|(idx, _)| idx)
            .collect();
        if alive.is_empty() {
            return None;
        }
        let idx = source.0 as usize % alive.len();
        Some(PlayerSlot(alive[idx] as u8))
    }

    fn incoming_garbage_outbound(
        incoming_notify: &[IncomingGarbage],
    ) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();
        for (slot, lines) in incoming_notify {
            if *lines == 0 {
                continue;
            }
            let pkt = PktIncomingGarbage {
                header: PacketHeader {
                    version: PROTOCOL_VERSION,
                    packet_type: PacketType::IncomingGarbage,
                    player_id: 0,
                },
                incoming_lines: *lines,
            };
            outbound.push(SimOutbound::ToPlayer(*slot, serialize_packet(&pkt)?));
        }
        Ok(outbound)
    }

    fn state_hash_outbound(
        &self,
        hashes: &[(PlayerSlot, u32)],
    ) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();
        for (idx, engine_opt) in self.engines.iter().enumerate() {
            if engine_opt.is_none() {
                continue;
            }
            let hash = hashes
                .iter()
                .find(|(slot, _)| slot.0 as usize == idx)
                .map_or(0, |(_, hash)| *hash);
            let pkt = PktStateHash {
                header: PacketHeader {
                    version: PROTOCOL_VERSION,
                    packet_type: PacketType::StateHash,
                    player_id: 0,
                },
                tick: self.tick,
                hash,
            };
            outbound.push(SimOutbound::ToPlayer(
                PlayerSlot(idx as u8),
                serialize_packet(&pkt)?,
            ));
        }
        Ok(outbound)
    }

    fn state_snapshot_outbound(&self) -> Result<Vec<SimOutbound>, SimError> {
        let mut outbound = Vec::new();
        for (idx, engine_opt) in self.engines.iter().enumerate() {
            let Some(engine) = engine_opt else {
                continue;
            };
            let snapshot = build_snapshot(engine, self.tick, self.seed);
            outbound.push(SimOutbound::ToPlayer(
                PlayerSlot(idx as u8),
                serialize_packet(&snapshot)?,
            ));
        }
        Ok(outbound)
    }

    fn player_state_packet(&self, slot: PlayerSlot) -> Result<Vec<u8>, SimError> {
        let idx = slot.0 as usize;
        let pkt = PktPlayerStateSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerStateSync,
                player_id: 0,
            },
            target_player_id: slot.0,
            alive: self.player_alive.get(idx).copied().unwrap_or(false),
            spectating: self.player_spectating.get(idx).is_some(),
            spectating_target: self
                .player_spectating
                .get(idx)
                .and_then(|slot| slot.map(|slot| slot.0)),
        };
        serialize_packet(&pkt)
    }

    fn game_over_packet(winner_id: u8) -> Result<Vec<u8>, SimError> {
        let pkt = PktGameOver {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::GameOver,
                player_id: 0,
            },
            winner_player_id: winner_id,
        };
        serialize_packet(&pkt)
    }

    fn reset_to_lobby(&mut self) {
        for engine in self.engines.iter_mut().flatten() {
            engine.reset(self.seed.0 as u32);
        }
        for alive in &mut self.player_alive {
            *alive = true;
        }
        for spec in &mut self.player_spectating {
            *spec = None;
        }
    }
}

fn to_engine_event(ev: &InputEvent) -> tetris_core::engine::InputEvent {
    tetris_core::engine::InputEvent {
        key: ev.key as u8,
        pressed: ev.pressed,
    }
}

fn serialize_packet<T: serde::Serialize>(pkt: &T) -> Result<Vec<u8>, SimError> {
    bincode::serialize(pkt).map_err(|_| SimError::PacketSerialize)
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

    #[test]
    fn api_adds_and_removes_player() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.remove_player(PlayerSlot(0));

        assert_eq!(sim.engine_count(), 0);
    }

    #[test]
    fn api_tick_applies_replay_input() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        let piece_before = sim.engine(PlayerSlot(0)).unwrap().state.piece;
        sim.enqueue_input(PlayerSlot(0), make_event(KeyAction::KeyHardDrop, true));
        sim.tick().unwrap();

        assert_ne!(sim.engine(PlayerSlot(0)).unwrap().state.piece, piece_before);
    }

    #[test]
    fn api_tick_returns_hash_outbound_at_interval() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        let mut outbound = Vec::new();
        for _ in 0..100 {
            outbound = sim.tick().unwrap();
        }

        assert!(!outbound.is_empty());
    }

    #[test]
    fn reconnect_returns_divergence_from_hash_ladder() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        for _ in 0..100 {
            sim.tick().unwrap();
        }
        let ack = sim.handle_reconnect(PlayerSlot(0), &[(TickNumber(0), 0xDEAD_BEEF)]);

        assert_eq!(ack.divergence_tick, TickNumber(0));
    }

    #[test]
    fn reconnect_replay_snapshot_fallback_returns_replay_when_buffer_covers_divergence() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        for _ in 0..100 {
            sim.tick().unwrap();
        }
        sim.enqueue_input(PlayerSlot(0), make_event(KeyAction::KeyLeft, true));
        sim.tick().unwrap();

        let ack = sim.handle_reconnect(PlayerSlot(0), &[(TickNumber(100), 0xDEAD_BEEF)]);

        assert_eq!(ack.divergence_tick, TickNumber(100));
        assert!(ack.snapshot.is_none());
        assert_eq!(ack.replay_events.len(), 1);
    }

    #[test]
    fn reconnect_replay_snapshot_fallback_returns_snapshot_when_replay_too_old() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        for tick in 0..2002 {
            let mut event = make_event(KeyAction::KeyLeft, true);
            event.tick = TickNumber(tick);
            sim.enqueue_input(PlayerSlot(0), event);
            sim.tick().unwrap();
        }

        let ack = sim.handle_reconnect(PlayerSlot(0), &[(TickNumber(0), 0xDEAD_BEEF)]);

        assert_eq!(ack.divergence_tick, TickNumber(0));
        assert!(ack.replay_events.is_empty());
        assert!(ack.snapshot.is_some());
    }

    #[test]
    fn reconnect_replay_snapshot_fallback_returns_noop_when_hashes_match() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        for _ in 0..100 {
            sim.tick().unwrap();
        }
        let hash = sim.hash_at(TickNumber(100)).unwrap();

        let ack = sim.handle_reconnect(PlayerSlot(0), &[(TickNumber(100), hash)]);

        assert_eq!(ack.divergence_tick, TickNumber(0));
        assert!(ack.replay_events.is_empty());
        assert!(ack.snapshot.is_none());
    }

    #[test]
    fn emits_replay_hash_snapshot() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.enqueue_input(PlayerSlot(0), make_event(KeyAction::KeyLeft, true));

        let first_tick = sim.tick().unwrap();

        assert!(first_tick.iter().any(|event| {
            let SimOutbound::ToPlayer(PlayerSlot(1), data) = event else {
                return false;
            };
            bincode::deserialize::<PktServerReplay>(data).is_ok()
        }));

        let mut interval_tick = Vec::new();
        for _ in 1..100 {
            interval_tick = sim.tick().unwrap();
        }

        assert!(interval_tick.iter().any(|event| {
            let SimOutbound::ToPlayer(_, data) = event else {
                return false;
            };
            bincode::deserialize::<PktStateHash>(data).is_ok()
        }));
        assert!(interval_tick.iter().any(|event| {
            let SimOutbound::ToPlayer(_, data) = event else {
                return false;
            };
            bincode::deserialize::<PktStateSnapshot>(data).is_ok()
        }));
    }

    #[test]
    fn lifecycle_game_over_broadcasts_winner() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.set_room_mode(RoomMode::Playing);
        sim.engine_mut(PlayerSlot(1)).unwrap().game_over = true;

        let outbound = sim.tick().unwrap();

        assert!(outbound.iter().any(|event| {
            let SimOutbound::Broadcast(data) = event else {
                return false;
            };
            bincode::deserialize::<PktGameOver>(data).is_ok_and(|pkt| pkt.winner_player_id == 0)
        }));
    }
}

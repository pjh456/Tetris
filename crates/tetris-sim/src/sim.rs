use std::collections::HashMap;

use tetris_core::engine::Engine;
use tetris_core::engine::RoomRules;
use tetris_protocol::newtypes::{PlayerSlot, Seed, TickNumber};
use tetris_protocol::protocol::{
    InputEvent, PacketHeader, PacketType, PktGameOver, PktIncomingGarbage, PktPlayerStatus,
    PktReconnectAck, PktServerReplay, PktStateHash, PktStateSnapshot,
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
    hash_ladder: HashMap<PlayerSlot, HashLadder>,
    pending_inputs: Vec<(PlayerSlot, InputEvent)>,
    player_alive: Vec<bool>,
    player_spectating: Vec<Option<PlayerSlot>>,
    player_paused: Vec<bool>,
    room_mode: RoomMode,
    game_over_countdown: u8,
    rules: RoomRules,
    elimination_order: Vec<PlayerSlot>,
    death_tick: Vec<Option<u32>>,
}

/// Per-player final stats used to build the standings table at match end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandingStat {
    pub slot: PlayerSlot,
    pub placement: u8,
    pub score: u32,
    pub lines: u32,
    pub survival_ticks: u32,
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
            hash_ladder: HashMap::new(),
            pending_inputs: Vec::new(),
            player_alive: Vec::new(),
            player_spectating: Vec::new(),
            player_paused: Vec::new(),
            room_mode: RoomMode::Lobby,
            game_over_countdown: 0,
            rules: RoomRules::default(),
            elimination_order: Vec::new(),
            death_tick: Vec::new(),
        }
    }

    pub fn add_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if self.engines.len() <= idx {
            self.engines.resize_with(idx + 1, || None);
            self.player_alive.resize_with(idx + 1, || true);
            self.player_spectating.resize_with(idx + 1, || None);
            self.player_paused.resize_with(idx + 1, || false);
            self.death_tick.resize_with(idx + 1, || None);
        }

        let mut engine = Engine::<10, 20>::new();
        engine.reset_with_rules(self.seed.0 as u32, self.rules);
        self.engines[idx] = Some(engine);
        self.player_alive[idx] = true;
        self.player_spectating[idx] = None;
        self.player_paused[idx] = false;
        self.death_tick[idx] = None;
        self.replay_buffers.insert(slot, ReplayBuffer::default());
    }

    pub fn remove_player(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if idx < self.engines.len() {
            self.engines[idx] = None;
            self.player_alive[idx] = false;
            self.player_spectating[idx] = None;
            self.player_paused[idx] = false;
        }
        self.replay_buffers.remove(&slot);
    }

    /// Pause a slot's engine: it keeps its state + hash but is skipped by
    /// `tick_engines` and never selected as an attack target. No-op if the slot
    /// has no engine. Used during the disconnect grace window to freeze the
    /// engine instead of ghost-ticking it.
    pub fn pause_slot(&mut self, slot: PlayerSlot) {
        let idx = slot.0 as usize;
        if self.engine(slot).is_some()
            && let Some(paused) = self.player_paused.get_mut(idx)
        {
            *paused = true;
        }
    }

    /// Resume a paused slot so its engine ticks again (called on reclaim).
    pub fn unpause_slot(&mut self, slot: PlayerSlot) {
        if let Some(paused) = self.player_paused.get_mut(slot.0 as usize) {
            *paused = false;
        }
    }

    pub fn is_paused(&self, slot: PlayerSlot) -> bool {
        self.player_paused
            .get(slot.0 as usize)
            .copied()
            .unwrap_or(false)
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

    /// Set per-room rules applied on subsequent (re)builds of player engines.
    /// Must be called before `add_player` / `restart_game` to take effect.
    pub fn set_rules(&mut self, rules: RoomRules) {
        self.rules = rules;
    }

    pub fn rules(&self) -> RoomRules {
        self.rules
    }

    /// Build final per-player standings at match end. Placement 1 = last
    /// survivor; earlier eliminations rank lower (higher number). Includes all
    /// players whose engine still exists (alive or eliminated, incl. bots).
    pub fn standings_stats(&self) -> Vec<StandingStat> {
        let total = self
            .engines
            .iter()
            .filter(|engine| engine.is_some())
            .count() as u8;
        // Elimination order restricted to current participants (earliest death first).
        let elim: Vec<usize> = self
            .elimination_order
            .iter()
            .map(|slot| slot.0 as usize)
            .filter(|idx| self.engines.get(*idx).is_some_and(Option::is_some))
            .collect();

        self.engines
            .iter()
            .enumerate()
            .filter_map(|(idx, engine_opt)| engine_opt.as_ref().map(|engine| (idx, engine)))
            .map(|(idx, engine)| {
                let alive = self.player_alive.get(idx).copied().unwrap_or(false);
                let placement = if alive {
                    1
                } else {
                    match elim.iter().position(|&e| e == idx) {
                        Some(p) => total.saturating_sub(p as u8),
                        None => total,
                    }
                };
                let survival_ticks = self
                    .death_tick
                    .get(idx)
                    .copied()
                    .flatten()
                    .unwrap_or(self.tick.0 as u32);
                StandingStat {
                    slot: PlayerSlot(idx as u8),
                    placement,
                    score: engine.scorer.score.min(u32::MAX as u64) as u32,
                    lines: engine.scorer.total_lines,
                    survival_ticks,
                }
            })
            .collect()
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

    pub fn hash_at(&self, slot: PlayerSlot, tick: TickNumber) -> Option<u32> {
        self.hash_ladder.get(&slot)?.get_hash_at(tick)
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
        let divergence = match self.hash_ladder.get(&slot) {
            Some(ladder) => ladder.find_divergence(client_hashes),
            None => client_hashes.first().map(|(tick, _)| *tick),
        };

        let Some(divergence_tick) = divergence else {
            return PktReconnectAck {
                header: PacketHeader::new(PacketType::ReconnectAck, 0),
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
                    header: PacketHeader::new(PacketType::ServerReplay, 0),
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
            header: PacketHeader::new(PacketType::ReconnectAck, 0),
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
            for (slot, hash) in &hashes {
                self.hash_ladder
                    .entry(*slot)
                    .or_default()
                    .insert(self.tick, *hash);
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
            header: PacketHeader::new(PacketType::ServerReplay, 0),
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
            if self.player_paused.get(idx).copied().unwrap_or(false) {
                continue;
            }
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
                header: PacketHeader::new(PacketType::ServerReplay, 0),
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
                self.elimination_order.push(*slot);
                if let Some(slot_death) = self.death_tick.get_mut(idx) {
                    *slot_death = Some(self.tick.0 as u32);
                }
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
                // 同 tick 全死（alive_count==0）时用 sentinel u8::MAX 表平局，
                // 不再把已死的 player 0 误判为赢家。
                let winner_id = self
                    .player_alive
                    .iter()
                    .enumerate()
                    .find(|(_, alive)| **alive)
                    .map_or(u8::MAX, |(idx, _)| idx as u8);
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
                engine.as_ref().is_some_and(|eng| !eng.game_over)
                    && *idx != source.0 as usize
                    && !self.player_paused.get(*idx).copied().unwrap_or(false)
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
                header: PacketHeader::new(PacketType::IncomingGarbage, 0),
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
                header: PacketHeader::new(PacketType::StateHash, 0),
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
        let pkt = PktPlayerStatus {
            header: PacketHeader::new(PacketType::PlayerStatus, 0),
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
            header: PacketHeader::new(PacketType::GameOver, 0),
            winner_player_id: winner_id,
        };
        serialize_packet(&pkt)
    }

    /// 复用 sim 开启新一局：重置所有引擎到新 seed、清存活/旁观/哈希/输入，进入 Playing。
    pub fn restart_game(&mut self, seed: Seed) {
        self.seed = seed;
        self.tick = TickNumber(0);
        self.hash_ladder.clear();
        self.pending_inputs.clear();
        self.elimination_order.clear();
        for slot_death in &mut self.death_tick {
            *slot_death = None;
        }
        for engine in self.engines.iter_mut().flatten() {
            engine.reset_with_rules(seed.0 as u32, self.rules);
        }
        for alive in &mut self.player_alive {
            *alive = true;
        }
        for spec in &mut self.player_spectating {
            *spec = None;
        }
        for paused in &mut self.player_paused {
            *paused = false;
        }
        self.game_over_countdown = 0;
        self.room_mode = RoomMode::Playing;
    }

    fn reset_to_lobby(&mut self) {
        self.elimination_order.clear();
        for slot_death in &mut self.death_tick {
            *slot_death = None;
        }
        for engine in self.engines.iter_mut().flatten() {
            engine.reset_with_rules(self.seed.0 as u32, self.rules);
        }
        for alive in &mut self.player_alive {
            *alive = true;
        }
        for spec in &mut self.player_spectating {
            *spec = None;
        }
        for paused in &mut self.player_paused {
            *paused = false;
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
        let hash = sim.hash_at(PlayerSlot(0), TickNumber(100)).unwrap();

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

    #[test]
    fn hash_ladder_stores_per_player_without_overwrite() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        // 让 p0 落子使其状态偏离 p1，确保同 tick 哈希不同。
        sim.enqueue_input(PlayerSlot(0), make_event(KeyAction::KeyHardDrop, true));
        for _ in 0..100 {
            sim.tick().unwrap();
        }
        let h0 = sim.hash_at(PlayerSlot(0), TickNumber(100)).unwrap();
        let h1 = sim.hash_at(PlayerSlot(1), TickNumber(100)).unwrap();
        assert_ne!(h0, h1, "同 tick 的多玩家哈希必须各自保留，不互相覆盖");
    }

    #[test]
    fn removed_player_not_attack_target() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.add_player(PlayerSlot(2));
        sim.remove_player(PlayerSlot(1));
        for _ in 0..20 {
            if let Some(target) = sim.route_attack_target(PlayerSlot(0)) {
                assert_ne!(target, PlayerSlot(1), "已移除的玩家不应被选为攻击目标");
            }
        }
    }

    #[test]
    fn double_death_winner_is_draw_sentinel() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.set_room_mode(RoomMode::Playing);
        sim.engine_mut(PlayerSlot(0)).unwrap().game_over = true;
        sim.engine_mut(PlayerSlot(1)).unwrap().game_over = true;

        let outbound = sim.tick().unwrap();

        assert!(outbound.iter().any(|event| {
            let SimOutbound::Broadcast(data) = event else {
                return false;
            };
            bincode::deserialize::<PktGameOver>(data)
                .is_ok_and(|pkt| pkt.winner_player_id == u8::MAX)
        }));
    }

    #[test]
    fn restart_game_resets_engines_and_seed() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.set_room_mode(RoomMode::Playing);
        sim.engine_mut(PlayerSlot(0)).unwrap().game_over = true;
        sim.restart_game(Seed(99));
        assert_eq!(sim.seed.0, 99);
        assert!(!sim.engine(PlayerSlot(0)).unwrap().game_over);
    }

    #[test]
    fn paused_slot_not_ticked() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.pause_slot(PlayerSlot(0));
        let p0_frozen = sim.engine(PlayerSlot(0)).unwrap().state_hash();
        let p1_before = sim.engine(PlayerSlot(1)).unwrap().state_hash();
        sim.enqueue_input(PlayerSlot(1), make_event(KeyAction::KeyHardDrop, true));
        for _ in 0..100 {
            sim.tick().unwrap();
        }

        assert_eq!(
            sim.engine(PlayerSlot(0)).unwrap().state_hash(),
            p0_frozen,
            "paused slot 的 engine 状态必须冻结"
        );
        assert_ne!(
            sim.engine(PlayerSlot(1)).unwrap().state_hash(),
            p1_before,
            "未暂停 slot 必须继续推进"
        );
    }

    #[test]
    fn paused_slot_not_attack_target() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.add_player(PlayerSlot(2));
        sim.pause_slot(PlayerSlot(1));
        for _ in 0..20 {
            if let Some(target) = sim.route_attack_target(PlayerSlot(0)) {
                assert_ne!(target, PlayerSlot(1), "暂停的玩家不应被选为攻击目标");
            }
            if let Some(target) = sim.route_attack_target(PlayerSlot(2)) {
                assert_ne!(target, PlayerSlot(1), "暂停的玩家不应被选为攻击目标");
            }
        }
    }

    #[test]
    fn unpause_slot_resumes_tick() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.pause_slot(PlayerSlot(0));
        for _ in 0..10 {
            sim.tick().unwrap();
        }
        let piece_frozen = sim.engine(PlayerSlot(0)).unwrap().state.piece;

        sim.unpause_slot(PlayerSlot(0));
        sim.enqueue_input(PlayerSlot(0), make_event(KeyAction::KeyHardDrop, true));
        sim.tick().unwrap();

        assert_ne!(
            sim.engine(PlayerSlot(0)).unwrap().state.piece,
            piece_frozen,
            "unpause 后 engine 必须恢复 tick"
        );
    }

    #[test]
    fn remove_player_clears_paused() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.pause_slot(PlayerSlot(0));
        assert!(sim.is_paused(PlayerSlot(0)));
        sim.remove_player(PlayerSlot(0));

        assert!(!sim.is_paused(PlayerSlot(0)));
        assert!(sim.engine(PlayerSlot(0)).is_none());
    }

    #[test]
    fn paused_slot_hash_ladder_unchanged() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.enqueue_input(PlayerSlot(1), make_event(KeyAction::KeyHardDrop, true));
        for _ in 0..50 {
            sim.tick().unwrap();
        }
        sim.pause_slot(PlayerSlot(0));
        let frozen = sim.engine(PlayerSlot(0)).unwrap().state_hash();
        for _ in 0..50 {
            sim.tick().unwrap();
        }

        assert_eq!(
            sim.engine(PlayerSlot(0)).unwrap().state_hash(),
            frozen,
            "paused engine 状态冻结"
        );
        assert_eq!(
            sim.hash_at(PlayerSlot(0), TickNumber(100)),
            Some(frozen),
            "paused slot 在 tick 100 记录的哈希应为冻结态哈希"
        );
    }

    #[test]
    fn rules_applied_to_new_engine() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.set_rules(RoomRules {
            start_level: 5,
            hold_enabled: true,
            initial_garbage_lines: 2,
        });
        sim.add_player(PlayerSlot(0));
        let engine = sim.engine(PlayerSlot(0)).unwrap();

        assert_eq!(engine.scorer.level, 5);
        assert_eq!(
            engine.state.board.rows.iter().filter(|r| **r != 0).count(),
            2,
            "开局应有 2 行初始垃圾"
        );
    }

    #[test]
    fn standings_ranks_by_elimination_order() {
        let mut sim = AuthoritativeSim::new(Seed(42));
        sim.add_player(PlayerSlot(0));
        sim.add_player(PlayerSlot(1));
        sim.add_player(PlayerSlot(2));
        sim.set_room_mode(RoomMode::Playing);

        // p1 死（仍有 p0/p2 存活，对局继续）
        sim.engine_mut(PlayerSlot(1)).unwrap().game_over = true;
        sim.tick().unwrap();
        // p2 死 → 仅 p0 存活 → 全局结束
        sim.engine_mut(PlayerSlot(2)).unwrap().game_over = true;
        sim.tick().unwrap();

        let stats = sim.standings_stats();
        let placement = |slot: u8| stats.iter().find(|s| s.slot.0 == slot).unwrap().placement;
        assert_eq!(placement(0), 1, "存活者第一");
        assert_eq!(placement(2), 2, "最后淘汰第二");
        assert_eq!(placement(1), 3, "最先淘汰第三");
    }
}

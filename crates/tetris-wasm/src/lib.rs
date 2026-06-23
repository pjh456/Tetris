use serde::{Deserialize, Serialize};
use tetris_core::engine::Action;
use tetris_core::engine::Engine;
use tetris_protocol::protocol::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use bincode::Options;

pub mod ai;
mod error;
mod input_buffer;
mod utils;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const OPPONENT_GRID_LEN: usize = 200;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const MAX_OPPONENTS: u8 = 8;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const MAX_PACKET_BYTES: u64 = 65536;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn deser<'de, T: serde::Deserialize<'de>>(data: &'de [u8]) -> Result<T, bincode::Error> {
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes()
        .with_limit(MAX_PACKET_BYTES)
        .deserialize::<T>(data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentInfo {
    pub player_id: u8,
    pub name: String,
    pub ready: bool,
    pub alive: bool,
    pub away: bool,
    pub is_host: bool,
    pub is_bot: bool,
    pub spectating: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplayerSnapshot {
    pub local_player_id: Option<u8>,
    pub room_code: Option<String>,
    pub countdown: Option<u8>,
    pub players: Vec<OpponentInfo>,
    pub opponents: Vec<OpponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplayerEvent {
    pub kind: String,
    pub room_code: Option<String>,
    pub player_id: Option<u8>,
    pub source_player_id: Option<u8>,
    pub countdown: Option<u8>,
    pub random_seed: Option<u32>,
    pub message: Option<String>,
    pub incoming_garbage_lines: Option<u8>,
    pub incoming_garbage_hole_x: Option<u8>,
    pub winner_player_id: Option<u8>,
    pub tick: Option<u64>,
    pub hash: Option<u32>,
    pub local_hash: Option<u32>,
    pub hash_match: Option<bool>,
    pub event_count: Option<usize>,
    pub resume_token: Option<String>,
    pub events: Vec<InputEvent>,
}

impl MultiplayerEvent {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn new(kind: impl Into<String>, room_code: Option<String>, countdown: Option<u8>) -> Self {
        Self {
            kind: kind.into(),
            room_code,
            player_id: None,
            source_player_id: None,
            countdown,
            random_seed: None,
            message: None,
            incoming_garbage_lines: None,
            incoming_garbage_hole_x: None,
            winner_player_id: None,
            tick: None,
            hash: None,
            local_hash: None,
            hash_match: None,
            event_count: None,
            resume_token: None,
            events: Vec::new(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[allow(dead_code)]
pub struct WebTetris {
    engine: Engine<10, 20>,
    has_hold: bool,
    grid_buf: Vec<u8>,
    opponent_engines: Vec<Engine<10, 20>>,
    opponent_grid_bufs: Vec<Vec<u8>>,
    room_player_infos: Vec<OpponentInfo>,
    opponent_infos: Vec<OpponentInfo>,
    room_code: Option<String>,
    local_player_id: Option<u8>,
    countdown: Option<u8>,
    last_event: Option<MultiplayerEvent>,
    input_buf: input_buffer::ClientInputBuffer,
    last_state_hash: Option<(tetris_protocol::newtypes::TickNumber, u32)>,
    mp_seed: u32,
}

// 纯 Rust 逻辑 + 私有 helper：对 wasm 导出，对 native 普通方法（tests 可见）。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl WebTetris {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(seed: u32) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset_with_level(seed, 1);
        let grid_buf = vec![0u8; 200];
        WebTetris {
            engine,
            has_hold: false,
            grid_buf,
            opponent_engines: Vec::new(),
            opponent_grid_bufs: Vec::new(),
            room_player_infos: Vec::new(),
            opponent_infos: Vec::new(),
            room_code: None,
            local_player_id: None,
            countdown: None,
            last_event: None,
            input_buf: input_buffer::ClientInputBuffer::new(),
            last_state_hash: None,
            mp_seed: 0,
        }
    }

    fn reset_common(&mut self, seed: u32, level: u32, clear_opponents: bool, clear_room: bool) {
        self.has_hold = false;
        self.engine.reset_with_level(seed, level);
        if clear_opponents {
            self.opponent_engines.clear();
            self.opponent_grid_bufs.clear();
        }
        if clear_room {
            self.room_player_infos.clear();
            self.opponent_infos.clear();
        }
        self.countdown = None;
        self.last_event = None;
        self.input_buf = input_buffer::ClientInputBuffer::new();
        self.last_state_hash = None;
    }

    pub fn reset(&mut self, seed: u32) {
        self.reset_common(seed, 1, true, true);
    }

    pub fn reset_multiplayer_game(&mut self, seed: u32) {
        // 清对手引擎/网格（杜绝跨局 stale），保留房间名册（由 snapshot 重建）。
        self.reset_common(seed, 1, true, false);
    }

    pub fn reset_with_level(&mut self, seed: u32, start_level: u32) {
        self.reset_common(seed, start_level.clamp(1, 15), true, true);
    }

    fn ensure_opponent_slot(&mut self, player_id: u8) -> Option<usize> {
        if player_id >= MAX_OPPONENTS {
            return None;
        }
        let idx = player_id as usize;
        if self
            .local_player_id
            .is_some_and(|local| idx == local as usize)
        {
            return None;
        }
        while self.opponent_engines.len() <= idx {
            // 必须 reset 脱离 Engine::new 的 game_over=true，否则 handle_action 被静默丢弃。
            let mut engine = Engine::<10, 20>::new();
            engine.reset(self.mp_seed);
            self.opponent_engines.push(engine);
            self.opponent_grid_bufs.push(vec![0u8; OPPONENT_GRID_LEN]);
        }
        Some(idx)
    }

    fn refresh_opponent_grid(&mut self, idx: usize) {
        if idx >= self.opponent_engines.len() || idx >= self.opponent_grid_bufs.len() {
            return;
        }
        let engine = &self.opponent_engines[idx];
        let ghost_y = tetris_core::rules::get_ghost_y(&engine.state);
        utils::fill_grid_buf(
            &engine.state,
            ghost_y,
            engine.game_over,
            &mut self.opponent_grid_bufs[idx],
        );
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter))]
    pub fn is_game_over(&self) -> bool {
        self.engine.game_over
    }

    pub fn grid_ptr(&self) -> *const u8 {
        self.grid_buf.as_ptr()
    }

    pub fn grid_len(&self) -> usize {
        self.grid_buf.len()
    }

    pub fn update_grid(&mut self) {
        let ghost_y = tetris_core::rules::get_ghost_y(&self.engine.state);
        utils::fill_grid_buf(
            &self.engine.state,
            ghost_y,
            self.engine.game_over,
            &mut self.grid_buf,
        );
    }

    pub fn get_hold(&self) -> i32 {
        if self.has_hold {
            self.engine.state.hold as i32
        } else {
            -1
        }
    }

    pub fn would_hit_wall(&self, dx: i8) -> bool {
        let test_x = self.engine.state.x + dx;
        !tetris_core::rules::can_place(
            &self.engine.state,
            test_x,
            self.engine.state.y,
            self.engine.state.rot,
        )
    }

    pub fn can_move(&self, dx: i8) -> bool {
        let test_x = self.engine.state.x + dx;
        tetris_core::rules::can_place(
            &self.engine.state,
            test_x,
            self.engine.state.y,
            self.engine.state.rot,
        )
    }

    pub fn get_last_clear_mask(&mut self) -> u64 {
        let mask = self.engine.state.last_clear_mask;
        self.engine.state.last_clear_mask = 0;
        mask
    }

    pub fn get_lock_timer(&self) -> u16 {
        self.engine.get_lock_timer().max(0) as u16
    }

    pub fn receive_garbage(&mut self, lines: u8, hole_x: u8) {
        // Clamp hole_x into [0, W-1]; an out-of-range hole would corrupt the garbage row.
        self.engine.add_pending_garbage(lines, hole_x.min(9), 0);
    }

    pub fn opponent_count(&self) -> u8 {
        self.opponent_infos.len() as u8
    }

    pub fn push_input_event(&mut self, key: u8, pressed: bool, subframe: f32) {
        let Some(action) = tetris_protocol::newtypes::KeyAction::from_u8(key) else {
            return;
        };
        self.input_buf.push(action, pressed, subframe);
    }

    pub fn advance_client_tick(&mut self) {
        self.input_buf.advance_tick();
    }

    pub fn should_flush_input(&self) -> bool {
        self.input_buf.should_flush()
    }

    pub fn get_state_hash(&self) -> u32 {
        self.engine.state_hash()
    }
}

// JS 边界方法：返回 JsValue/Uint8Array，仅 wasm 编译。
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebTetris {
    pub fn tick(&mut self, delta_ms: u32) -> JsValue {
        let result = self.engine.tick(delta_ms);
        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    pub fn handle_action(&mut self, action_val: u8) -> JsValue {
        let action = Action::from_u8(action_val);
        let result = self.engine.handle_action(action);
        if action == Action::Hold {
            self.has_hold = true;
        }
        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }

    pub fn get_grid(&self) -> js_sys::Uint8Array {
        let ghost_y = tetris_core::rules::get_ghost_y(&self.engine.state);
        let grid = utils::build_grid(&self.engine.state, ghost_y, self.engine.game_over);
        let arr = js_sys::Uint8Array::new_with_length(200);
        for (i, &v) in grid.iter().enumerate() {
            arr.set_index(i as u32, v);
        }
        arr
    }

    pub fn get_next(&self) -> js_sys::Uint8Array {
        let next = utils::build_next(&self.engine.state);
        let arr = js_sys::Uint8Array::new_with_length(5);
        for (i, &v) in next.iter().enumerate() {
            arr.set_index(i as u32, v);
        }
        arr
    }

    pub fn get_last_hard_drop_info(&self) -> JsValue {
        let info = serde_wasm_bindgen::to_value(&utils::HardDropInfo {
            cols: self.engine.state.last_harddrop_cols,
            start_y: self.engine.state.last_harddrop_start_y,
            end_y: self.engine.state.last_harddrop_end_y,
            piece: self.engine.state.last_harddrop_piece as u8,
        });
        info.unwrap_or(JsValue::NULL)
    }

    pub fn get_hud_data(&self) -> JsValue {
        let s = &self.engine.scorer;
        let data = utils::HudData {
            score: s.score,
            level: s.level,
            lines: s.total_lines,
            combo: s.combo,
            b2b: s.b2b_count,
            tspin: s.tspin_count,
            all_clear: s.all_clear_count,
        };
        serde_wasm_bindgen::to_value(&data).unwrap_or(JsValue::NULL)
    }

    pub fn get_game_stats(&self) -> JsValue {
        let s = &self.engine.scorer;
        let data = utils::GameStats {
            score: s.score,
            lines: s.total_lines,
            level: s.level,
            game_time_ms: s.game_time_ms,
            max_combo: s.max_combo,
            tspin_count: s.tspin_count,
            total_pieces: s.total_pieces,
        };
        serde_wasm_bindgen::to_value(&data).unwrap_or(JsValue::NULL)
    }

    pub fn parse_packet(&mut self, data: &[u8]) -> JsValue {
        match self.apply_packet(data) {
            Some(event) => serde_wasm_bindgen::to_value(&event).unwrap_or(JsValue::NULL),
            None => JsValue::NULL,
        }
    }

    pub fn get_multiplayer_snapshot(&self) -> JsValue {
        let snapshot = MultiplayerSnapshot {
            local_player_id: self.local_player_id,
            room_code: self.room_code.clone(),
            countdown: self.countdown,
            players: self.room_player_infos.clone(),
            opponents: self.opponent_infos.clone(),
        };
        serde_wasm_bindgen::to_value(&snapshot).unwrap_or(JsValue::NULL)
    }

    pub fn consume_last_multiplayer_event(&mut self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.last_event.take()).unwrap_or(JsValue::NULL)
    }

    pub fn make_join_room_packet(
        &self,
        room_code: String,
        player_name: String,
    ) -> js_sys::Uint8Array {
        let pkt = PktJoinRoom {
            header: PacketHeader::new(PacketType::JoinRoom, self.local_player_id.unwrap_or(0)),
            room_code,
            player_name,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_player_ready_packet(&self, ready: bool) -> js_sys::Uint8Array {
        let pkt = PktPlayerReady {
            header: PacketHeader::new(PacketType::PlayerReady, self.local_player_id.unwrap_or(0)),
            ready,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_chat_message_packet(
        &self,
        message: String,
        timestamp: String,
    ) -> js_sys::Uint8Array {
        let pkt = PktChatMessage {
            header: PacketHeader::new(PacketType::ChatMessage, self.local_player_id.unwrap_or(0)),
            message,
            timestamp,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_add_bot_packet(&self, temperature: f32) -> js_sys::Uint8Array {
        let pkt = PktAddBot {
            header: PacketHeader::new(PacketType::AddBot, self.local_player_id.unwrap_or(0)),
            temperature,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_kick_player_packet(&self, target_id: u8) -> js_sys::Uint8Array {
        let pkt = PktKickPlayer {
            header: PacketHeader::new(PacketType::KickPlayer, self.local_player_id.unwrap_or(0)),
            target_player_id: target_id,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_remove_bot_packet(&self, target_id: u8) -> js_sys::Uint8Array {
        let pkt = PktRemoveBot {
            header: PacketHeader::new(PacketType::RemoveBot, self.local_player_id.unwrap_or(0)),
            target_player_id: target_id,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn get_opponent_grid(&self, player_id: u8) -> js_sys::Uint8Array {
        let idx = player_id as usize;
        if idx >= self.opponent_grid_bufs.len() {
            return js_sys::Uint8Array::new_with_length(200);
        }
        let buf = &self.opponent_grid_bufs[idx];
        js_sys::Uint8Array::from(buf.as_slice())
    }

    pub fn get_opponent_info(&self, index: u8) -> JsValue {
        let idx = index as usize;
        if idx >= self.opponent_infos.len() {
            return JsValue::NULL;
        }
        serde_wasm_bindgen::to_value(&self.opponent_infos[idx]).unwrap_or(JsValue::NULL)
    }

    pub fn flush_input_buffer(&mut self) -> JsValue {
        let events = self.input_buf.flush();
        serde_wasm_bindgen::to_value(&events).unwrap_or(JsValue::NULL)
    }

    pub fn make_replay_packet(&self, events_js: JsValue) -> js_sys::Uint8Array {
        let Ok(events) = serde_wasm_bindgen::from_value::<Vec<InputEvent>>(events_js) else {
            return js_sys::Uint8Array::new_with_length(0);
        };
        let pkt = PktReplay {
            header: PacketHeader::new(PacketType::Replay, self.local_player_id.unwrap_or(0)),
            start_tick: events
                .first()
                .map_or(self.input_buf.current_tick(), |event| event.tick),
            events,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_resume_packet(
        &self,
        socket_id: String,
        resume_token: String,
    ) -> js_sys::Uint8Array {
        let pkt = PktResume {
            header: PacketHeader::new(PacketType::Resume, self.local_player_id.unwrap_or(0)),
            socket_id,
            resume_token,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_reconnect_packet(&self) -> js_sys::Uint8Array {
        packet_to_uint8_array(&self.build_reconnect_packet())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
}

#[cfg(target_arch = "wasm32")]
fn packet_to_uint8_array<T: Serialize>(packet: &T) -> js_sys::Uint8Array {
    match bincode::serialize(packet) {
        Ok(bytes) => js_sys::Uint8Array::from(bytes.as_slice()),
        Err(_) => js_sys::Uint8Array::new_with_length(0),
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
impl WebTetris {
    fn rebuild_opponents(&mut self) {
        self.opponent_infos = match self.local_player_id {
            Some(local_player_id) => self
                .room_player_infos
                .iter()
                .filter(|player| player.player_id != local_player_id)
                .cloned()
                .collect(),
            None => Vec::new(),
        };
    }

    /// Build the reconnect packet. Pure logic so it is unit-testable on native
    /// (the wasm `make_reconnect_packet` wrapper only adds the `Uint8Array` bridge).
    fn build_reconnect_packet(&self) -> PktReconnect {
        use tetris_protocol::newtypes::TickNumber;
        // `last_good_tick` must reflect the real tick of the last server-confirmed
        // state hash — never the unconditional (TickNumber(0), local_hash) entry,
        // which previously pinned it to 0 and produced a wrong diverged_tick.
        let last_good_tick = self.last_state_hash.map_or(TickNumber(0), |(tick, _)| tick);
        let mut client_hashes = Vec::new();
        if let Some((tick, hash)) = self.last_state_hash {
            client_hashes.push((tick, hash));
        }
        // Current local hash is an extra ladder entry; it does NOT define last_good_tick.
        client_hashes.push((TickNumber(0), self.engine.state_hash()));
        PktReconnect {
            header: PacketHeader::new(PacketType::Reconnect, self.local_player_id.unwrap_or(0)),
            last_good_tick,
            client_hashes,
        }
    }

    fn apply_local_state_snapshot(&mut self, pkt: &PktStateSnapshot) {
        self.engine.reset(pkt.seed.0 as u32);
        for (idx, row) in pkt.board_rows.iter().copied().enumerate().take(20) {
            self.engine.state.board.rows[idx] = row;
        }
        self.engine.state.piece = pkt.piece;
        self.engine.state.rot = pkt.rot;
        self.engine.state.x = pkt.x;
        self.engine.state.y = pkt.y;
        self.engine.state.hold = pkt.hold;
        self.engine.state.hold_used = pkt.hold_used;
        self.engine.state.next = pkt.next;
        self.engine.state.rng = pkt.rng_state;
        self.engine.state.combo = pkt.combo;
        self.engine.state.b2b = pkt.b2b;
        self.engine.state.pending_garbage = pkt.pending_garbage;
        self.engine.game_over = false;
        self.has_hold = pkt.hold_used;
    }

    fn apply_server_replay(&mut self, pkt: &PktServerReplay) -> bool {
        let player_id = pkt.source_player.0;
        let Some(idx) = self.ensure_opponent_slot(player_id) else {
            return false;
        };
        for event in &pkt.events {
            let action = Action::from_u8(event.key as u8);
            if event.pressed {
                self.opponent_engines[idx].handle_action(action);
            }
        }
        if pkt.ige_garbage_lines > 0 {
            let engine = &mut self.opponent_engines[idx];
            engine.state.pending_garbage = engine
                .state
                .pending_garbage
                .saturating_add(pkt.ige_garbage_lines);
            engine.garbage_hole_x = pkt.ige_hole_x;
        }
        self.refresh_opponent_grid(idx);
        true
    }

    fn apply_packet(&mut self, data: &[u8]) -> Option<MultiplayerEvent> {
        // Distinguish the three parse-failure classes (decode / version / unknown
        // type) via dedicated event kinds, so the JS layer can observe them
        // instead of silently dropping every failure to null.
        let Ok(header) = deser::<PacketHeader>(data) else {
            return Some(MultiplayerEvent::new(
                "parse_error_decode",
                self.room_code.clone(),
                None,
            ));
        };
        if header.version != PROTOCOL_VERSION {
            return Some(MultiplayerEvent::new(
                "parse_error_version",
                self.room_code.clone(),
                None,
            ));
        }

        let event = match header.packet_type {
            PacketType::Batch => {
                let batch: PktBatch = deser(data).ok()?;
                let mut last_event = None;
                for inner in &batch.packets {
                    last_event = self.apply_packet(inner).or(last_event);
                }
                return last_event;
            }
            PacketType::ServerAccept => {
                let pkt: PktServerAccept = deser(data).ok()?;
                self.local_player_id = Some(pkt.assigned_player_id);
                self.rebuild_opponents();
                let mut event =
                    MultiplayerEvent::new("server_accept", self.room_code.clone(), None);
                event.player_id = Some(pkt.assigned_player_id);
                event.resume_token = Some(pkt.resume_token);
                event
            }
            PacketType::RoomSnapshot => {
                let pkt: PktRoomSnapshot = deser(data).ok()?;
                self.room_code = Some(pkt.room_code.clone());
                self.room_player_infos = pkt
                    .players
                    .iter()
                    .map(|player| OpponentInfo {
                        player_id: player.player_id,
                        name: player.name.clone(),
                        ready: player.ready,
                        alive: player.alive,
                        away: player.away,
                        is_host: player.is_host,
                        is_bot: player.is_bot,
                        spectating: false,
                    })
                    .collect();
                self.rebuild_opponents();
                MultiplayerEvent::new("room_snapshot", self.room_code.clone(), self.countdown)
            }
            PacketType::GameStart => {
                let pkt: PktGameStart = deser(data).ok()?;
                self.mp_seed = pkt.random_seed;
                self.countdown = None;
                let mut event = MultiplayerEvent::new("game_start", self.room_code.clone(), None);
                event.random_seed = Some(pkt.random_seed);
                event
            }
            PacketType::ChatMessage => {
                let pkt: PktChatMessage = deser(data).ok()?;
                let mut event =
                    MultiplayerEvent::new("chat", self.room_code.clone(), self.countdown);
                event.player_id = Some(pkt.header.player_id);
                event.message = Some(pkt.message);
                event
            }
            PacketType::StartCountdown => {
                let pkt: PktStartCountdown = deser(data).ok()?;
                self.countdown = Some(pkt.remaining_secs);
                MultiplayerEvent::new("countdown", self.room_code.clone(), self.countdown)
            }
            PacketType::CountdownCancel => {
                let _pkt: PktCountdownCancel = deser(data).ok()?;
                self.countdown = None;
                MultiplayerEvent::new("countdown_cancel", self.room_code.clone(), None)
            }
            PacketType::StateHash => {
                let pkt: PktStateHash = deser(data).ok()?;
                let local_hash = self.engine.state_hash();
                let hash_match = local_hash == pkt.hash;
                self.last_state_hash = Some((pkt.tick, pkt.hash));
                let mut event = MultiplayerEvent::new(
                    if hash_match {
                        "state_hash"
                    } else {
                        "resync_required"
                    },
                    self.room_code.clone(),
                    self.countdown,
                );
                event.tick = Some(pkt.tick.0);
                event.hash = Some(pkt.hash);
                event.local_hash = Some(local_hash);
                event.hash_match = Some(hash_match);
                event
            }
            PacketType::StateSnapshot => {
                let pkt: PktStateSnapshot = deser(data).ok()?;
                self.apply_local_state_snapshot(&pkt);
                let mut event =
                    MultiplayerEvent::new("state_snapshot", self.room_code.clone(), self.countdown);
                event.player_id = self.local_player_id;
                event.tick = Some(pkt.tick.0);
                event
            }
            PacketType::ServerReplay => {
                let pkt: PktServerReplay = deser(data).ok()?;
                if !self.apply_server_replay(&pkt) {
                    return None;
                }
                let mut event =
                    MultiplayerEvent::new("server_replay", self.room_code.clone(), self.countdown);
                event.source_player_id = Some(pkt.source_player.0);
                event.incoming_garbage_lines = Some(pkt.ige_garbage_lines);
                event.incoming_garbage_hole_x = Some(pkt.ige_hole_x);
                event.event_count = Some(pkt.events.len());
                event.events = pkt.events;
                event
            }
            PacketType::ReconnectAck => {
                let pkt: PktReconnectAck = deser(data).ok()?;
                let replay_count = pkt.replay_events.len();
                for replay in &pkt.replay_events {
                    self.apply_server_replay(replay);
                }
                if let Some(snapshot) = &pkt.snapshot {
                    self.apply_local_state_snapshot(snapshot);
                }
                let mut event =
                    MultiplayerEvent::new("reconnect_ack", self.room_code.clone(), self.countdown);
                event.tick = Some(pkt.divergence_tick.0);
                event.event_count = Some(replay_count);
                event
            }
            PacketType::IncomingGarbage => {
                let pkt: PktIncomingGarbage = deser(data).ok()?;
                let mut event = MultiplayerEvent::new(
                    "incoming_garbage",
                    self.room_code.clone(),
                    self.countdown,
                );
                event.incoming_garbage_lines = Some(pkt.incoming_lines);
                event
            }
            PacketType::PlayerStatus => {
                let pkt: PktPlayerStatus = deser(data).ok()?;
                if let Some(info) = self
                    .opponent_infos
                    .iter_mut()
                    .find(|player| player.player_id == pkt.target_player_id)
                {
                    info.alive = pkt.alive;
                    info.spectating = pkt.spectating;
                }
                let mut event =
                    MultiplayerEvent::new("player_state", self.room_code.clone(), self.countdown);
                event.player_id = Some(pkt.target_player_id);
                event
            }
            PacketType::GameOver => {
                let pkt: PktGameOver = deser(data).ok()?;
                let mut event =
                    MultiplayerEvent::new("game_over", self.room_code.clone(), self.countdown);
                event.winner_player_id = Some(pkt.winner_player_id);
                event
            }
            PacketType::KickPlayer => {
                let pkt: PktKickPlayer = deser(data).ok()?;
                let mut event =
                    MultiplayerEvent::new("kicked", self.room_code.clone(), self.countdown);
                event.player_id = Some(pkt.target_player_id);
                event
            }
            PacketType::RemoveBot => {
                let pkt: PktRemoveBot = deser(data).ok()?;
                let mut event =
                    MultiplayerEvent::new("bot_removed", self.room_code.clone(), self.countdown);
                event.player_id = Some(pkt.target_player_id);
                event
            }
            _ => {
                return Some(MultiplayerEvent::new(
                    "parse_error_unknown",
                    self.room_code.clone(),
                    None,
                ));
            }
        };

        self.last_event = Some(event.clone());
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tetris_core::types::{Piece, Rot};
    use tetris_protocol::newtypes::{KeyAction, PlayerSlot, Seed, TickNumber};

    fn packet_bytes<T: Serialize>(pkt: &T) -> Vec<u8> {
        bincode::serialize(pkt).unwrap()
    }

    #[test]
    fn test_web_tetris_new() {
        let wt = WebTetris::new(42);
        assert!(!wt.engine.game_over);
        assert!(!wt.has_hold);
        assert_eq!(wt.grid_buf.len(), 200);
    }

    #[test]
    fn test_web_tetris_deterministic_seed() {
        let wt1 = WebTetris::new(42);
        let wt2 = WebTetris::new(42);
        assert_eq!(wt1.engine.state.piece, wt2.engine.state.piece);
    }

    #[test]
    fn test_web_tetris_different_seeds() {
        let wt1 = WebTetris::new(1);
        let wt2 = WebTetris::new(999);
        let same_piece = wt1.engine.state.piece == wt2.engine.state.piece;
        let same_next = wt1.engine.state.next == wt2.engine.state.next;
        assert!(
            !(same_piece && same_next),
            "different seeds should produce different states"
        );
    }

    #[test]
    fn packet_parse_room_snapshot_waits_for_local_player_id() {
        let mut wt = WebTetris::new(42);
        let snapshot = PktRoomSnapshot {
            header: PacketHeader::new(PacketType::RoomSnapshot, 0),
            room_code: "ABCD".into(),
            players: vec![
                RoomPlayerSnapshot {
                    player_id: 0,
                    name: "Alice".into(),
                    ready: true,
                    alive: true,
                    away: false,
                    is_host: true,
                    is_bot: false,
                },
                RoomPlayerSnapshot {
                    player_id: 1,
                    name: "Bob".into(),
                    ready: true,
                    alive: true,
                    away: false,
                    is_host: false,
                    is_bot: false,
                },
            ],
        };

        let event = wt.apply_packet(&packet_bytes(&snapshot)).unwrap();

        assert_eq!(event.kind, "room_snapshot");
        assert!(wt.opponent_infos.is_empty());

        let accept = PktServerAccept {
            header: PacketHeader::new(PacketType::ServerAccept, 0),
            assigned_player_id: 0,
            max_players: 2,
            resume_token: "tok".into(),
        };
        wt.apply_packet(&packet_bytes(&accept)).unwrap();
        wt.apply_packet(&packet_bytes(&snapshot)).unwrap();

        assert_eq!(wt.local_player_id, Some(0));
        assert_eq!(wt.opponent_infos.len(), 1);
        assert_eq!(wt.opponent_infos[0].player_id, 1);
    }

    #[test]
    fn countdown_cancel_clears_countdown() {
        let mut wt = WebTetris::new(42);
        let countdown = PktStartCountdown {
            header: PacketHeader::new(PacketType::StartCountdown, 0),
            remaining_secs: 3,
        };
        wt.apply_packet(&packet_bytes(&countdown)).unwrap();
        assert_eq!(wt.countdown, Some(3));

        let cancel = PktCountdownCancel {
            header: PacketHeader::new(PacketType::CountdownCancel, 0),
        };
        let event = wt.apply_packet(&packet_bytes(&cancel)).unwrap();

        assert_eq!(event.kind, "countdown_cancel");
        assert_eq!(wt.countdown, None);
    }

    #[test]
    fn reset_multiplayer_game_preserves_room_metadata() {
        let mut wt = WebTetris::new(42);
        let accept = PktServerAccept {
            header: PacketHeader::new(PacketType::ServerAccept, 0),
            assigned_player_id: 0,
            max_players: 2,
            resume_token: "tok".into(),
        };
        let snapshot = PktRoomSnapshot {
            header: PacketHeader::new(PacketType::RoomSnapshot, 0),
            room_code: "ABCD".into(),
            players: vec![
                RoomPlayerSnapshot {
                    player_id: 0,
                    name: "Alice".into(),
                    ready: true,
                    alive: true,
                    away: false,
                    is_host: true,
                    is_bot: false,
                },
                RoomPlayerSnapshot {
                    player_id: 1,
                    name: "AI 1".into(),
                    ready: true,
                    alive: true,
                    away: false,
                    is_host: false,
                    is_bot: true,
                },
            ],
        };
        wt.apply_packet(&packet_bytes(&accept)).unwrap();
        wt.apply_packet(&packet_bytes(&snapshot)).unwrap();

        wt.reset_multiplayer_game(99);

        assert_eq!(wt.local_player_id, Some(0));
        assert_eq!(wt.room_code.as_deref(), Some("ABCD"));
        assert_eq!(wt.room_player_infos.len(), 2);
        assert_eq!(wt.opponent_infos.len(), 1);
        assert_eq!(wt.opponent_infos[0].name, "AI 1");
    }

    #[test]
    fn packet_parse_state_hash_reports_mismatch() {
        let mut wt = WebTetris::new(42);
        let pkt = PktStateHash {
            header: PacketHeader::new(PacketType::StateHash, 0),
            tick: TickNumber(100),
            hash: 0xDEAD_BEEF,
        };

        let event = wt.apply_packet(&packet_bytes(&pkt)).unwrap();

        assert_eq!(event.kind, "resync_required");
        assert_eq!(event.tick, Some(100));
        assert_eq!(event.hash, Some(0xDEAD_BEEF));
        assert_eq!(event.hash_match, Some(false));
    }

    #[test]
    fn reconnect_packet_uses_real_last_state_hash_tick() {
        let mut wt = WebTetris::new(42);
        wt.local_player_id = Some(1);
        let hash_pkt = PktStateHash {
            header: PacketHeader::new(PacketType::StateHash, 0),
            tick: TickNumber(50),
            hash: 0xABCD,
        };
        wt.apply_packet(&packet_bytes(&hash_pkt)).unwrap();

        let pkt = wt.build_reconnect_packet();

        // last_good_tick reflects the confirmed hash tick (50), not the old恒-0 bug.
        assert_eq!(pkt.last_good_tick, TickNumber(50));
        assert_eq!(pkt.header.player_id, 1);
        assert!(pkt.client_hashes.iter().any(|(t, _)| *t == TickNumber(50)));
    }

    #[test]
    fn reconcile_state_snapshot_applies_local_authority() {
        let mut wt = WebTetris::new(42);
        let pkt = PktStateSnapshot {
            header: PacketHeader::new(PacketType::StateSnapshot, 0),
            tick: TickNumber(7),
            board_rows: vec![0x3ff; 20],
            piece: Piece::O,
            rot: Rot::R90,
            x: 4,
            y: 5,
            hold: Piece::T,
            hold_used: true,
            next: [Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z],
            rng_state: 99,
            combo: 2,
            b2b: true,
            pending_garbage: 3,
            seed: Seed(123),
        };

        let event = wt.apply_packet(&packet_bytes(&pkt)).unwrap();

        assert_eq!(event.kind, "state_snapshot");
        assert_eq!(event.tick, Some(7));
        assert_eq!(wt.engine.state.piece, Piece::O);
        assert_eq!(wt.engine.state.rot, Rot::R90);
        assert_eq!(wt.engine.state.board.rows[0], 0x3ff);
        assert_eq!(wt.engine.state.pending_garbage, 3);
    }

    #[test]
    fn packet_parse_server_replay_updates_opponent() {
        let mut wt = WebTetris::new(42);
        wt.local_player_id = Some(0);
        let pkt = PktServerReplay {
            header: PacketHeader::new(PacketType::ServerReplay, 0),
            source_player: PlayerSlot(1),
            events: vec![InputEvent {
                key: KeyAction::KeyLeft,
                pressed: true,
                tick: TickNumber(1),
                subframe: 0.0,
            }],
            ige_garbage_lines: 2,
            ige_hole_x: 4,
        };

        let event = wt.apply_packet(&packet_bytes(&pkt)).unwrap();

        assert_eq!(event.kind, "server_replay");
        assert_eq!(event.source_player_id, Some(1));
        assert_eq!(event.event_count, Some(1));
        assert_eq!(event.incoming_garbage_lines, Some(2));
        assert_eq!(wt.opponent_engines[1].state.pending_garbage, 2);
    }

    #[test]
    fn opponent_engine_not_game_over_after_game_start_replay() {
        let mut wt = WebTetris::new(42);
        wt.local_player_id = Some(0);
        let start = PktGameStart {
            header: PacketHeader::new(PacketType::GameStart, 0),
            random_seed: 7,
        };
        wt.apply_packet(&packet_bytes(&start)).unwrap();
        let replay = PktServerReplay {
            header: PacketHeader::new(PacketType::ServerReplay, 0),
            source_player: PlayerSlot(1),
            events: vec![InputEvent {
                key: KeyAction::KeyHardDrop,
                pressed: true,
                tick: TickNumber(1),
                subframe: 0.0,
            }],
            ige_garbage_lines: 0,
            ige_hole_x: 0,
        };
        wt.apply_packet(&packet_bytes(&replay)).unwrap();
        // 对手引擎已脱离 Engine::new 的 game_over=true，操作不再被静默丢弃。
        assert!(!wt.opponent_engines[1].game_over);
    }

    #[test]
    fn reset_multiplayer_game_clears_opponent_engines() {
        let mut wt = WebTetris::new(42);
        wt.local_player_id = Some(0);
        let start = PktGameStart {
            header: PacketHeader::new(PacketType::GameStart, 0),
            random_seed: 5,
        };
        wt.apply_packet(&packet_bytes(&start)).unwrap();
        let replay = PktServerReplay {
            header: PacketHeader::new(PacketType::ServerReplay, 0),
            source_player: PlayerSlot(1),
            events: vec![],
            ige_garbage_lines: 0,
            ige_hole_x: 0,
        };
        wt.apply_packet(&packet_bytes(&replay)).unwrap();
        assert!(!wt.opponent_engines.is_empty());
        wt.reset_multiplayer_game(99);
        assert!(wt.opponent_engines.is_empty());
    }
}

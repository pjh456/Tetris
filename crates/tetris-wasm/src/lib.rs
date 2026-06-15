use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use tetris_core::engine::Action;
use tetris_core::engine::Engine;
#[cfg(target_arch = "wasm32")]
use tetris_protocol::protocol::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod error;
mod input_buffer;
mod utils;

#[cfg(target_arch = "wasm32")]
const OPPONENT_GRID_LEN: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentInfo {
    pub player_id: u8,
    pub name: String,
    pub ready: bool,
    pub alive: bool,
    pub away: bool,
    pub is_host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplayerSnapshot {
    pub local_player_id: u8,
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
    pub countdown: Option<u8>,
    pub random_seed: Option<u32>,
    pub message: Option<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
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
    local_player_id: u8,
    countdown: Option<u8>,
    last_event: Option<MultiplayerEvent>,
    input_buf: input_buffer::ClientInputBuffer,
    last_state_hash: Option<(tetris_protocol::newtypes::TickNumber, u32)>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebTetris {
    #[wasm_bindgen(constructor)]
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
            local_player_id: 0,
            countdown: None,
            last_event: None,
            input_buf: input_buffer::ClientInputBuffer::new(),
            last_state_hash: None,
        }
    }

    pub fn reset(&mut self, seed: u32) {
        self.has_hold = false;
        self.engine.reset_with_level(seed, 1);
        self.opponent_engines.clear();
        self.opponent_grid_bufs.clear();
        self.room_player_infos.clear();
        self.opponent_infos.clear();
        self.countdown = None;
        self.last_event = None;
    }

    pub fn reset_with_level(&mut self, seed: u32, start_level: u32) {
        self.has_hold = false;
        self.engine.reset_with_level(seed, start_level.clamp(1, 15));
        self.opponent_engines.clear();
        self.opponent_grid_bufs.clear();
        self.room_player_infos.clear();
        self.opponent_infos.clear();
        self.countdown = None;
        self.last_event = None;
    }

    fn ensure_opponent_slot(&mut self, player_id: u8) -> Option<usize> {
        let idx = player_id as usize;
        if idx == self.local_player_id as usize {
            return None;
        }
        while self.opponent_engines.len() <= idx {
            self.opponent_engines.push(Engine::<10, 20>::new());
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

    fn apply_state_sync(&mut self, player_id: u8, pkt: &PktStateSync) {
        let Some(idx) = self.ensure_opponent_slot(player_id) else {
            return;
        };
        let engine = &mut self.opponent_engines[idx];
        for (row_idx, row_val) in pkt.board_rows.iter().enumerate().take(20) {
            engine.state.board.rows[row_idx] = *row_val;
        }
        engine.state.piece = pkt.piece;
        engine.state.rot = pkt.rot;
        engine.state.x = pkt.x;
        engine.state.y = pkt.y;
        engine.state.hold = pkt.hold;
        engine.state.hold_used = pkt.hold_used;
        engine.state.next = [
            pkt.next[0],
            pkt.next[1],
            pkt.next[2],
            pkt.next[0],
            pkt.next[1],
        ];
        engine.state.pending_garbage = pkt.pending_garbage;
        engine.state.rng = pkt.rng_state;
        engine.game_over = false;
        self.refresh_opponent_grid(idx);
    }

    fn apply_delta_sync(&mut self, player_id: u8, pkt: &PktDeltaSync) {
        let Some(idx) = self.ensure_opponent_slot(player_id) else {
            return;
        };
        let engine = &mut self.opponent_engines[idx];
        for &(row_idx, row_val) in &pkt.changed_rows {
            let row_idx = row_idx as usize;
            if row_idx < 20 {
                engine.state.board.rows[row_idx] = row_val;
            }
        }
        engine.state.piece = pkt.piece;
        engine.state.rot = pkt.rot;
        engine.state.x = pkt.x;
        engine.state.y = pkt.y;
        engine.state.hold = pkt.hold;
        engine.state.hold_used = pkt.hold_used;
        engine.state.next = [
            pkt.next[0],
            pkt.next[1],
            pkt.next[2],
            pkt.next[0],
            pkt.next[1],
        ];
        engine.game_over = false;
        self.refresh_opponent_grid(idx);
    }

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

    #[wasm_bindgen(getter)]
    pub fn is_game_over(&self) -> bool {
        self.engine.game_over
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
        if !self.has_hold {
            -1
        } else {
            self.engine.state.hold as i32
        }
    }

    pub fn get_next(&self) -> js_sys::Uint8Array {
        let next = utils::build_next(&self.engine.state);
        let arr = js_sys::Uint8Array::new_with_length(5);
        for (i, &v) in next.iter().enumerate() {
            arr.set_index(i as u32, v);
        }
        arr
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

    pub fn get_last_clear_mask(&mut self) -> u32 {
        let mask = self.engine.state.last_clear_mask;
        self.engine.state.last_clear_mask = 0;
        mask
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

    pub fn get_lock_timer(&self) -> u16 {
        self.engine.get_lock_timer().max(0) as u16
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
        use bincode::deserialize;
        let header: PacketHeader = match deserialize(data) {
            Ok(h) => h,
            Err(_) => return JsValue::NULL,
        };
        if header.version != PROTOCOL_VERSION {
            return JsValue::NULL;
        }
        match header.packet_type {
            PacketType::ServerAccept => {
                let pkt: PktServerAccept = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.local_player_id = pkt.assigned_player_id;
                self.last_event = Some(MultiplayerEvent {
                    kind: "server_accept".into(),
                    room_code: self.room_code.clone(),
                    player_id: Some(pkt.assigned_player_id),
                    countdown: None,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::RoomSnapshot => {
                let pkt: PktRoomSnapshot = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
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
                    })
                    .collect();
                self.opponent_infos = self
                    .room_player_infos
                    .iter()
                    .filter(|player| player.player_id != self.local_player_id)
                    .cloned()
                    .collect();
                self.last_event = Some(MultiplayerEvent {
                    kind: "room_snapshot".into(),
                    room_code: self.room_code.clone(),
                    player_id: None,
                    countdown: self.countdown,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::GameStart => {
                let pkt: PktGameStart = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.countdown = None;
                self.last_event = Some(MultiplayerEvent {
                    kind: "game_start".into(),
                    room_code: self.room_code.clone(),
                    player_id: None,
                    countdown: None,
                    random_seed: Some(pkt.random_seed),
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::StateSync => {
                let pkt: PktStateSync = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.apply_state_sync(header.player_id, &pkt);
                self.last_event = Some(MultiplayerEvent {
                    kind: "state_sync".into(),
                    room_code: self.room_code.clone(),
                    player_id: Some(header.player_id),
                    countdown: self.countdown,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::DeltaSync => {
                let pkt: PktDeltaSync = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.apply_delta_sync(header.player_id, &pkt);
                self.last_event = Some(MultiplayerEvent {
                    kind: "delta_sync".into(),
                    room_code: self.room_code.clone(),
                    player_id: Some(header.player_id),
                    countdown: self.countdown,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::ChatMessage => {
                let pkt: PktChatMessage = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.last_event = Some(MultiplayerEvent {
                    kind: "chat".into(),
                    room_code: self.room_code.clone(),
                    player_id: Some(pkt.header.player_id),
                    countdown: self.countdown,
                    random_seed: None,
                    message: Some(pkt.message.clone()),
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::StartCountdown => {
                let pkt: PktStartCountdown = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.countdown = Some(pkt.remaining_secs);
                self.last_event = Some(MultiplayerEvent {
                    kind: "countdown".into(),
                    room_code: self.room_code.clone(),
                    player_id: None,
                    countdown: Some(pkt.remaining_secs),
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::StateHash => {
                let pkt: PktStateHash = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.last_state_hash = Some((pkt.tick, pkt.hash));
                JsValue::NULL
            }
            PacketType::StateSnapshot => {
                let pkt: PktStateSnapshot = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                self.apply_state_snapshot(header.player_id, &pkt);
                self.last_event = Some(MultiplayerEvent {
                    kind: "state_snapshot".into(),
                    room_code: self.room_code.clone(),
                    player_id: Some(header.player_id),
                    countdown: self.countdown,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            PacketType::ServerReplay => {
                let pkt: PktServerReplay = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                let player_id = pkt.source_player.0;
                let Some(idx) = self.ensure_opponent_slot(player_id) else {
                    return JsValue::NULL;
                };
                for event in &pkt.events {
                    let action = Action::from_u8(event.key as u8);
                    if event.pressed {
                        self.opponent_engines[idx].handle_action(action);
                    }
                }
                if pkt.ige_garbage_lines > 0 {
                    self.opponent_engines[idx]
                        .state
                        .pending_garbage = self.opponent_engines[idx]
                        .state
                        .pending_garbage
                        .saturating_add(pkt.ige_garbage_lines);
                }
                self.refresh_opponent_grid(idx);
                JsValue::NULL
            }
            PacketType::ReconnectAck => {
                let pkt: PktReconnectAck = match deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                for replay in &pkt.replay_events {
                    let player_id = replay.source_player.0;
                    let Some(idx) = self.ensure_opponent_slot(player_id) else {
                        continue;
                    };
                    for event in &replay.events {
                        let action = Action::from_u8(event.key as u8);
                        if event.pressed {
                            self.opponent_engines[idx].handle_action(action);
                        }
                    }
                    self.refresh_opponent_grid(idx);
                }
                self.last_event = Some(MultiplayerEvent {
                    kind: "reconnect_ack".into(),
                    room_code: self.room_code.clone(),
                    player_id: None,
                    countdown: self.countdown,
                    random_seed: None,
                    message: None,
                });
                serde_wasm_bindgen::to_value(&self.last_event).unwrap_or(JsValue::NULL)
            }
            _ => JsValue::NULL,
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
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::JoinRoom,
                player_id: self.local_player_id,
            },
            room_code,
            player_name,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_player_ready_packet(&self, ready: bool) -> js_sys::Uint8Array {
        let pkt = PktPlayerReady {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::PlayerReady,
                player_id: self.local_player_id,
            },
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
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::ChatMessage,
                player_id: self.local_player_id,
            },
            message,
            timestamp,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn make_state_sync_packet(&self) -> js_sys::Uint8Array {
        let pkt = PktStateSync {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::StateSync,
                player_id: self.local_player_id,
            },
            board_rows: self.engine.state.board.rows.to_vec(),
            piece: self.engine.state.piece,
            rot: self.engine.state.rot,
            x: self.engine.state.x,
            y: self.engine.state.y,
            hold: self.engine.state.hold,
            hold_used: self.engine.state.hold_used,
            next: [
                self.engine.state.next[0],
                self.engine.state.next[1],
                self.engine.state.next[2],
            ],
            pending_garbage: self.engine.state.pending_garbage,
            rng_state: self.engine.state.rng,
        };
        packet_to_uint8_array(&pkt)
    }

    pub fn get_opponent_grid(&self, player_id: u8) -> js_sys::Uint8Array {
        let idx = player_id as usize;
        if idx >= self.opponent_grid_bufs.len() {
            return js_sys::Uint8Array::new(&js_sys::Uint8Array::new_with_length(200));
        }
        let buf = &self.opponent_grid_bufs[idx];
        js_sys::Uint8Array::from(buf.as_slice())
    }

    pub fn opponent_count(&self) -> u8 {
        self.opponent_infos.len() as u8
    }

    pub fn get_opponent_info(&self, index: u8) -> JsValue {
        let idx = index as usize;
        if idx >= self.opponent_infos.len() {
            return JsValue::NULL;
        }
        serde_wasm_bindgen::to_value(&self.opponent_infos[idx]).unwrap_or(JsValue::NULL)
    }

    fn apply_state_snapshot(&mut self, player_id: u8, pkt: &PktStateSnapshot) {
        let Some(idx) = self.ensure_opponent_slot(player_id) else {
            return;
        };
        let engine = &mut self.opponent_engines[idx];
        for (row_idx, row_val) in pkt.board_rows.iter().enumerate().take(20) {
            engine.state.board.rows[row_idx] = *row_val;
        }
        engine.state.piece = pkt.piece;
        engine.state.rot = pkt.rot;
        engine.state.x = pkt.x;
        engine.state.y = pkt.y;
        engine.state.hold = pkt.hold;
        engine.state.hold_used = pkt.hold_used;
        engine.state.next = pkt.next;
        engine.state.rng = pkt.rng_state;
        engine.state.combo = pkt.combo;
        engine.state.b2b = pkt.b2b;
        engine.state.pending_garbage = pkt.pending_garbage;
        engine.game_over = false;
        self.refresh_opponent_grid(idx);
    }

    pub fn push_input_event(&mut self, key: u8, pressed: bool, subframe: f32) {
        let action = tetris_protocol::newtypes::KeyAction::from_u8(key);
        self.input_buf.push(action, pressed, subframe);
    }

    pub fn flush_input_buffer(&mut self) -> JsValue {
        let events = self.input_buf.flush();
        serde_wasm_bindgen::to_value(&events).unwrap_or(JsValue::NULL)
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

    pub fn make_replay_packet(&self, events_js: JsValue) -> js_sys::Uint8Array {
        let Ok(events) = serde_wasm_bindgen::from_value::<Vec<InputEvent>>(events_js) else {
            return js_sys::Uint8Array::new_with_length(0);
        };
        let pkt = PktReplay {
            header: PacketHeader {
                version: PROTOCOL_VERSION,
                packet_type: PacketType::Replay,
                player_id: self.local_player_id,
            },
            events,
            start_tick: self.input_buf.current_tick(),
        };
        packet_to_uint8_array(&pkt)
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

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub struct WebTetris {
    #[allow(dead_code)]
    pub engine: Engine<10, 20>,
    has_hold: bool,
    grid_buf: Vec<u8>,
    opponent_engines: Vec<Engine<10, 20>>,
    opponent_grid_bufs: Vec<Vec<u8>>,
    opponent_infos: Vec<OpponentInfo>,
    room_code: Option<String>,
    local_player_id: u8,
    countdown: Option<u8>,
    last_event: Option<MultiplayerEvent>,
    input_buf: input_buffer::ClientInputBuffer,
    last_state_hash: Option<(tetris_protocol::newtypes::TickNumber, u32)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WebTetris {
    pub fn new(seed: u32) -> Self {
        let mut engine = Engine::<10, 20>::new();
        engine.reset_with_level(seed, 1);
        WebTetris {
            engine,
            has_hold: false,
            grid_buf: vec![0u8; 200],
            opponent_engines: Vec::new(),
            opponent_grid_bufs: Vec::new(),
            opponent_infos: Vec::new(),
            room_code: None,
            local_player_id: 0,
            countdown: None,
            last_event: None,
            input_buf: input_buffer::ClientInputBuffer::new(),
            last_state_hash: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

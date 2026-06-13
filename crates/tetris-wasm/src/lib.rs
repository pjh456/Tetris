use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use tetris_core::engine::Action;
use tetris_core::engine::Engine;
#[cfg(target_arch = "wasm32")]
use tetris_net::protocol::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod error;
mod utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpponentInfo {
    pub player_id: u8,
    pub name: String,
    pub alive: bool,
    pub away: bool,
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
    opponent_infos: Vec<OpponentInfo>,
    room_code: Option<String>,
    local_player_id: u8,
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
            opponent_infos: Vec::new(),
            room_code: None,
            local_player_id: 0,
        }
    }

    pub fn reset(&mut self, seed: u32) {
        self.has_hold = false;
        self.engine.reset_with_level(seed, 1);
        self.opponent_engines.clear();
        self.opponent_grid_bufs.clear();
        self.opponent_infos.clear();
    }

    pub fn reset_with_level(&mut self, seed: u32, start_level: u32) {
        self.has_hold = false;
        self.engine.reset_with_level(seed, start_level.clamp(1, 15));
        self.opponent_engines.clear();
        self.opponent_grid_bufs.clear();
        self.opponent_infos.clear();
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
        let header: PacketHeader = match bincode::deserialize(data) {
            Ok(h) => h,
            Err(_) => return JsValue::NULL,
        };
        if header.version != PROTOCOL_VERSION {
            return JsValue::NULL;
        }
        match header.packet_type {
            PacketType::GameStart => {
                self.opponent_engines.clear();
                self.opponent_grid_bufs.clear();
                self.opponent_infos.clear();
                JsValue::from_str("game_start")
            }
            PacketType::StateSync => {
                let pkt: PktStateSync = match bincode::deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                let pid = pkt.header.player_id as usize;
                while self.opponent_engines.len() <= pid {
                    let mut e = Engine::<10, 20>::new();
                    e.reset_with_level(0, 1);
                    self.opponent_engines.push(e);
                    self.opponent_grid_bufs.push(vec![0u8; 200]);
                }
                JsValue::from_str("state_sync")
            }
            PacketType::DeltaSync => match serde_wasm_bindgen::to_value(&header) {
                Ok(v) => v,
                Err(_) => JsValue::NULL,
            },
            PacketType::ChatMessage => {
                let pkt: PktChatMessage = match bincode::deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                serde_wasm_bindgen::to_value(&pkt).unwrap_or(JsValue::NULL)
            }
            PacketType::PlayerReady => JsValue::from_str("player_ready"),
            PacketType::StartCountdown => {
                let pkt: PktStartCountdown = match bincode::deserialize(data) {
                    Ok(p) => p,
                    Err(_) => return JsValue::NULL,
                };
                serde_wasm_bindgen::to_value(&pkt).unwrap_or(JsValue::NULL)
            }
            _ => JsValue::NULL,
        }
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
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_memory() -> JsValue {
    wasm_bindgen::memory()
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

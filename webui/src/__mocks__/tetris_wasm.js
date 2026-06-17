import { vi } from 'vitest';

export class WebTetris {
  constructor(seed) {
    this._seed = seed;
    this._game_over = false;
    this._input_events = [];
    this._client_tick = 0;
  }
  reset(seed) {
    this._seed = seed;
  }
  tick() {
    return {};
  }
  handle_action() {
    return {};
  }
  get is_game_over() {
    return this._game_over;
  }
  get_grid() {
    return new Uint8Array(200);
  }
  grid_ptr() {
    return 0;
  }
  grid_len() {
    return 200;
  }
  update_grid() {}
  get_hold() {
    return -1;
  }
  get_next() {
    return new Uint8Array(5);
  }
  would_hit_wall() {
    return false;
  }
  can_move() {
    return true;
  }
  get_last_clear_mask() {
    return 0;
  }
  get_last_hard_drop_info() {
    return { cols: 0, start_y: 0, end_y: 0, piece: 0 };
  }
  get_lock_timer() {
    return 0;
  }
  get_hud_data() {
    return { score: 0, level: 1, lines: 0, combo: 0, b2b: 0, tspin: 0, all_clear: 0 };
  }
  get_game_stats() {
    return {
      score: 0,
      lines: 0,
      level: 1,
      game_time_ms: 0,
      max_combo: 0,
      tspin_count: 0,
      total_pieces: 0,
    };
  }
  push_input_event(key, pressed, subframe) {
    this._input_events.push({ key, pressed, tick: this._client_tick, subframe });
  }
  advance_client_tick() {
    this._client_tick += 1;
  }
  should_flush_input() {
    return this._client_tick % 30 === 0;
  }
  flush_input_buffer() {
    const events = this._input_events;
    this._input_events = [];
    return events;
  }
  make_replay_packet(events) {
    return events.length === 0 ? new Uint8Array(0) : new Uint8Array([23, events.length]);
  }
  make_state_sync_packet() {
    return new Uint8Array([25]);
  }
  get_multiplayer_snapshot() {
    return { local_player_id: 0, room_code: 'ABCD', countdown: null, players: [], opponents: [] };
  }
  opponent_count() {
    return 0;
  }
  get_opponent_grid() {
    return new Uint8Array(200);
  }
  get_opponent_info() {
    return null;
  }
  parse_packet() {
    return null;
  }
}

export function wasm_memory() {
  return { buffer: new ArrayBuffer(65536) };
}

export default vi.fn(async () => {});

import { vi } from 'vitest';

export class WebTetris {
  constructor(seed) {
    this._seed = seed;
    this._game_over = false;
  }
  reset(seed) { this._seed = seed; }
  tick() { return {}; }
  handle_action(_action_val) { return {}; }
  get is_game_over() { return this._game_over; }
  get_grid() { return new Uint8Array(200); }
  grid_ptr() { return 0; }
  grid_len() { return 200; }
  update_grid() {}
  get_hold() { return -1; }
  get_next() { return new Uint8Array(5); }
  would_hit_wall(_dx) { return false; }
  can_move(_dx) { return true; }
  get_last_clear_mask() { return 0; }
  get_last_hard_drop_info() { return { cols: 0, start_y: 0, end_y: 0, piece: 0 }; }
  get_lock_timer() { return 0; }
  get_hud_data() { return { score: 0, level: 1, lines: 0, combo: 0, b2b: 0, tspin: 0, all_clear: 0 }; }
  get_game_stats() { return { score: 0, lines: 0, level: 1, game_time_ms: 0, max_combo: 0, tspin_count: 0, total_pieces: 0 }; }
}

export function wasm_memory() {
  return { buffer: new ArrayBuffer(65536) };
}

export default vi.fn(async () => {});

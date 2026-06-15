import type { WebTetris } from '../../wasm/tetris_wasm.js';

const FLUSH_INTERVAL_TICKS = 30;

export class InputBuffer {
  private tick = 0;
  private last_flush_tick = 0;
  private wasm: WebTetris;

  constructor(wasm: WebTetris) {
    this.wasm = wasm;
  }

  push(key: number, pressed: boolean) {
    this.wasm.push_input_event(key, pressed, 0);
  }

  advance_tick() {
    this.tick += 1;
    this.wasm.advance_client_tick();
  }

  should_flush(): boolean {
    return this.tick - this.last_flush_tick >= FLUSH_INTERVAL_TICKS;
  }

  flush(): Uint8Array | null {
    const events_js = this.wasm.flush_input_buffer();
    if (!events_js) return null;
    this.last_flush_tick = this.tick;
    return this.wasm.make_replay_packet(events_js);
  }
}

import init, { WebTetris, wasm_memory } from '../../wasm/tetris_wasm.js';
import { create_button } from './dom';

interface LoadingElement extends HTMLElement {
  _cleanup?: () => void;
}

let instance: WebTetris | null = null;
let init_promise: Promise<WebTetris> | null = null;
let grid_view: Uint8Array | null = null;

function create_grid_view(wasm: WebTetris): Uint8Array {
  const memory = wasm_memory() as unknown as WebAssembly.Memory;
  return new Uint8Array(memory.buffer, wasm.grid_ptr(), wasm.grid_len());
}

function require_instance(): WebTetris {
  if (!instance) throw new Error('WASM not initialized');
  return instance;
}

export function get_grid_view(): Uint8Array {
  const i = require_instance();
  // Recreate on every call: WASM memory may have grown, detaching old view.
  i.update_grid();
  grid_view = create_grid_view(i);
  return grid_view;
}

const TIPS = [
  'Press Space to Hard Drop',
  'T-Spin deals double damage',
  'Clear 4 lines for a Tetris!',
  'Hold pieces with Tab',
  'Combos increase attack power',
  'Back-to-Back boosts damage',
];

export function wasm_loading_screen(): HTMLElement {
  const el = document.createElement('div');
  el.style.cssText =
    'display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:24px;';
  el.innerHTML = `
    <div style="font-family:var(--font-display);font-size:48px;color:var(--color-accent);text-shadow:0 0 20px var(--color-accent);letter-spacing:6px;">TETRIS</div>
    <div style="width:200px;height:4px;background:var(--color-panel-border);border-radius:2px;overflow:hidden;">
      <div class="loading-bar" style="height:100%;width:0%;background:var(--color-accent);animation:loading-progress 3s ease-in-out infinite;"></div>
    </div>
    <div class="loading-tip" style="color:var(--color-muted);font-size:14px;min-height:20px;"></div>
  `;

  const style = document.createElement('style');
  style.textContent = `@keyframes loading-progress{0%{width:0%}50%{width:80%}100%{width:100%}}`;
  el.prepend(style);

  const tip_el = el.querySelector('.loading-tip') as HTMLElement;
  let idx = Math.floor(Math.random() * TIPS.length);
  tip_el.textContent = TIPS[idx];
  const timer = setInterval(() => {
    idx = (idx + 1) % TIPS.length;
    tip_el.textContent = TIPS[idx];
  }, 3000);

  (el as LoadingElement)._cleanup = () => clearInterval(timer);
  return el;
}

export function wasm_error_screen(msg: string): HTMLElement {
  const el = document.createElement('div');
  el.style.cssText =
    'display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:16px;';
  el.innerHTML = `
    <div style="font-family:var(--font-display);font-size:26px;color:var(--color-destructive);">Failed to Load</div>
    <div style="color:var(--color-muted);font-size:14px;max-width:400px;text-align:center;">
      Game engine could not be initialized. Check your connection and try again.
    </div>
  `;
  const msg_el = document.createElement('div');
  msg_el.style.cssText = 'color:var(--color-muted);font-size:12px;opacity:0.6;';
  msg_el.textContent = msg;
  el.appendChild(msg_el);
  const btn = create_button('Retry Loading', { onClick: () => location.reload() });
  el.appendChild(btn);
  return el;
}

export async function init_wasm(container?: HTMLElement): Promise<WebTetris> {
  if (instance) return instance;
  if (init_promise) return init_promise;

  const do_init = async (): Promise<WebTetris> => {
    const target = container || document.getElementById('app');
    if (!target) throw new Error('No container element');

    const loading = wasm_loading_screen();
    target.innerHTML = '';
    target.appendChild(loading);

    const cleanup_timer = () => {
      (loading as LoadingElement)._cleanup?.();
    };

    try {
      await init();
      const seed = Date.now() >>> 0;
      instance = new WebTetris(seed);
      grid_view = create_grid_view(instance);
      cleanup_timer();
      return instance;
    } catch (err) {
      cleanup_timer();
      target.innerHTML = '';
      target.appendChild(wasm_error_screen(err instanceof Error ? err.message : 'Unknown error'));
      throw err;
    } finally {
      init_promise = null;
    }
  };

  init_promise = do_init();
  return init_promise;
}

export function get_wasm(): WebTetris {
  return require_instance();
}

export type { WebTetris };

export function reset_wasm(): WebTetris {
  const i = require_instance();
  const seed = Date.now() >>> 0;
  i.reset(seed);
  grid_view = create_grid_view(i);
  return i;
}

export function reset_multiplayer_wasm(seed: number): WebTetris {
  const i = require_instance();
  i.reset_multiplayer_game(seed >>> 0);
  grid_view = create_grid_view(i);
  return i;
}

export function get_opponent_grid_view(player_id: number): Uint8Array {
  return require_instance().get_opponent_grid(player_id);
}

export function get_opponent_count(): number {
  if (!instance) return 0;
  return instance.opponent_count();
}

export function get_opponent_info(index: number): Record<string, unknown> | null {
  if (!instance) return null;
  const result = instance.get_opponent_info(index);
  if (result === null || result === undefined) return null;
  return result as Record<string, unknown>;
}

export function get_opponent_player_grid(player_id: number): Uint8Array {
  return require_instance().get_opponent_grid(player_id);
}

export type MultiplayerPlayer = {
  player_id: number;
  name: string;
  ready: boolean;
  alive: boolean;
  away: boolean;
  is_host: boolean;
};

export type MultiplayerSnapshot = {
  local_player_id: number | null;
  room_code: string | null;
  countdown: number | null;
  players: MultiplayerPlayer[];
  opponents: MultiplayerPlayer[];
};

export type MultiplayerEvent = {
  kind: string;
  room_code?: string | null;
  player_id?: number | null;
  source_player_id?: number | null;
  countdown?: number | null;
  random_seed?: number | null;
  message?: string | null;
  incoming_garbage_lines?: number | null;
  incoming_garbage_hole_x?: number | null;
  winner_player_id?: number | null;
  tick?: number | null;
  hash?: number | null;
  local_hash?: number | null;
  hash_match?: boolean | null;
  event_count?: number | null;
  resume_token?: string | null;
  events?: unknown[];
};

export function get_multiplayer_snapshot(): MultiplayerSnapshot | null {
  if (!instance) return null;
  return instance.get_multiplayer_snapshot() as MultiplayerSnapshot;
}

export function consume_last_multiplayer_event(): MultiplayerEvent | null {
  if (!instance) return null;
  const result = instance.consume_last_multiplayer_event();
  if (result === null || result === undefined) return null;
  return result as MultiplayerEvent | null;
}

export function make_join_room_packet(room: string, player_name: string): Uint8Array {
  return require_instance().make_join_room_packet(room, player_name);
}

export function make_player_ready_packet(ready: boolean): Uint8Array {
  return require_instance().make_player_ready_packet(ready);
}

export function make_chat_message_packet(message: string): Uint8Array {
  return require_instance().make_chat_message_packet(message, new Date().toISOString());
}

export function make_add_bot_packet(temperature: number): Uint8Array {
  return require_instance().make_add_bot_packet(temperature);
}

export function push_input_event(key: number, pressed: boolean, subframe = 0): void {
  require_instance().push_input_event(key, pressed, subframe);
}

export function advance_client_tick(): void {
  require_instance().advance_client_tick();
}

export function should_flush_input(): boolean {
  return require_instance().should_flush_input();
}

export function flush_input_buffer(): unknown {
  return require_instance().flush_input_buffer();
}

export function make_replay_packet(events: unknown): Uint8Array {
  return require_instance().make_replay_packet(events);
}

export function make_resume_packet(socket_id: string, resume_token: string): Uint8Array {
  return require_instance().make_resume_packet(socket_id, resume_token);
}

export function make_reconnect_packet(): Uint8Array {
  return require_instance().make_reconnect_packet();
}

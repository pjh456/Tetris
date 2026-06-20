import { WasmAi } from '../../wasm/tetris_wasm.js';
import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { get_multiplayer_ws } from './multiplayer';
import { make_add_bot_packet } from './wasm';
import type { WsClient } from './ws_client';

type AttackResult = {
  damage?: number;
  hole_x?: number;
};

const AI_DECIDE_INTERVAL_MS = 180;
const DEFAULT_AI_TEMP = 0;

let local_ai: WasmAi | null = null;
let decide_accumulator = 0;

export function add_ai_opponent(_wasm: WebTetris, temperature = DEFAULT_AI_TEMP): 'local' | 'net' {
  const ws = get_multiplayer_ws();
  if (ws) {
    add_multiplayer_ai_opponent(ws, temperature);
    return 'net';
  }

  local_ai = new WasmAi(Date.now() >>> 0);
  local_ai.set_temperature(temperature);
  decide_accumulator = 0;
  return 'local';
}

export function add_multiplayer_ai_opponent(ws: WsClient, temperature = DEFAULT_AI_TEMP) {
  if (!ws.is_open()) {
    return false;
  }
  ws.send(make_add_bot_packet(temperature));
  return true;
}

export function has_local_ai_opponent(): boolean {
  return local_ai !== null;
}

export function reset_local_ai_opponent() {
  local_ai = null;
  decide_accumulator = 0;
}

export function decide(): number | null {
  if (!local_ai) return null;
  return local_ai.decide();
}

export function tick_ai_opponent(wasm: WebTetris, delta_ms: number, human_attack?: unknown) {
  if (!local_ai) return;

  forward_attack_to_ai(human_attack);
  local_ai.tick(Math.max(0, Math.floor(delta_ms)));
  decide_accumulator += delta_ms;
  if (decide_accumulator >= AI_DECIDE_INTERVAL_MS) {
    local_ai.decide();
    decide_accumulator = 0;
  }

  const garbage = local_ai.drain_pending_garbage();
  for (let i = 0; i + 1 < garbage.length; i += 2) {
    wasm.receive_garbage(garbage[i], garbage[i + 1]);
  }
}

export function get_ai_opponent_grid(): Uint8Array | null {
  return local_ai?.get_grid() ?? null;
}

function forward_attack_to_ai(attack: unknown) {
  if (!local_ai || !is_attack_result(attack) || !attack.damage || attack.damage <= 0) {
    return;
  }
  local_ai.receive_garbage(attack.damage, attack.hole_x ?? 0);
}

function is_attack_result(value: unknown): value is AttackResult {
  return typeof value === 'object' && value !== null && 'damage' in value;
}

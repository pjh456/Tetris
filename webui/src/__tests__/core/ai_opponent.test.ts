import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../wasm/tetris_wasm.js', () => import('../../__mocks__/tetris_wasm.js'));

import { WasmAi } from '../../../wasm/tetris_wasm.js';
import {
  add_ai_opponent,
  add_multiplayer_ai_opponent,
  decide,
  has_local_ai_opponent,
  reset_local_ai_opponent,
  tick_ai_opponent,
} from '../../core/ai_opponent';
import { set_multiplayer_ws } from '../../core/multiplayer';
import { init_wasm } from '../../core/wasm';

beforeEach(() => {
  vi.restoreAllMocks();
  reset_local_ai_opponent();
  set_multiplayer_ws(null);
});

describe('ai_opponent', () => {
  it('uses local wasm inference for single player', async () => {
    const wasm = await init_wasm(document.createElement('div'));

    const mode = add_ai_opponent(wasm);
    const action = decide();

    expect(mode).toBe('local');
    expect(has_local_ai_opponent()).toBe(true);
    expect(action).toBeGreaterThanOrEqual(0);
    expect(action).toBeLessThan(40);
  });

  it('routes multiplayer add AI through net bot request', async () => {
    const wasm = await init_wasm(document.createElement('div'));
    const ws = { send: vi.fn(), is_open: () => true };

    set_multiplayer_ws(ws as never);
    const mode = add_ai_opponent(wasm);

    expect(mode).toBe('net');
    expect(ws.send).toHaveBeenCalledWith(new Uint8Array([34]));
    expect(has_local_ai_opponent()).toBe(false);
  });

  it('can send add bot request from lobby websocket', () => {
    const ws = { send: vi.fn(), is_open: () => true };

    add_multiplayer_ai_opponent(ws as never);

    expect(ws.send).toHaveBeenCalledWith(new Uint8Array([34]));
  });

  it('feeds AI outbound garbage into the human board', async () => {
    const wasm = await init_wasm(document.createElement('div'));
    const receive_garbage = vi.spyOn(wasm, 'receive_garbage');
    vi.spyOn(WasmAi.prototype, 'drain_pending_garbage').mockReturnValue([2, 4]);

    add_ai_opponent(wasm);
    tick_ai_opponent(wasm, 16);

    expect(receive_garbage).toHaveBeenCalledWith(2, 4);
  });
});

import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../wasm/tetris_wasm.js', () => import('../../__mocks__/tetris_wasm.js'));

import { add_multiplayer_ai_opponent } from '../../core/ai_opponent';
import { init_wasm } from '../../core/wasm';

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('ai_opponent', () => {
  it('sends an add-bot request over an open websocket', async () => {
    await init_wasm(document.createElement('div'));
    const ws = { send: vi.fn(), is_open: () => true };

    const result = add_multiplayer_ai_opponent(ws as never);

    expect(result).toBe(true);
    expect(ws.send).toHaveBeenCalledWith(new Uint8Array([34]));
  });

  it('does nothing when the websocket is not open', () => {
    const ws = { send: vi.fn(), is_open: () => false };

    const result = add_multiplayer_ai_opponent(ws as never);

    expect(result).toBe(false);
    expect(ws.send).not.toHaveBeenCalled();
  });
});

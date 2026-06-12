import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../../../wasm/tetris_wasm.js', () => import('../../__mocks__/tetris_wasm.js'));

import { init_wasm, get_wasm, reset_wasm } from '../../core/wasm';

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('init_wasm', () => {
  it('returns a WebTetris instance', async () => {
    const container = document.createElement('div');
    const wasm = await init_wasm(container);
    expect(wasm).toBeDefined();
    expect(wasm.is_game_over).toBe(false);
  });
});

describe('get_wasm', () => {
  it('returns instance after init', async () => {
    const container = document.createElement('div');
    await init_wasm(container);
    const wasm = get_wasm();
    expect(wasm).toBeDefined();
  });
});

describe('reset_wasm', () => {
  it('resets and returns instance', async () => {
    const container = document.createElement('div');
    await init_wasm(container);
    const wasm = reset_wasm();
    expect(wasm).toBeDefined();
  });
});

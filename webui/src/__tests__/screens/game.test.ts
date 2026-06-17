import { beforeEach, describe, it, expect, vi } from 'vitest';

vi.mock('../../../wasm/tetris_wasm.js', () => import('../../__mocks__/tetris_wasm.js'));

vi.stubGlobal(
  'AudioContext',
  vi.fn(() => ({
    createOscillator: vi.fn(() => ({
      connect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
      frequency: { setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn() },
      type: 'sine',
    })),
    createGain: vi.fn(() => ({
      connect: vi.fn(),
      gain: { setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn(), value: 1 },
    })),
    createBuffer: vi.fn(() => ({ getChannelData: vi.fn(() => new Float32Array(44100)) })),
    createBufferSource: vi.fn(() => ({
      connect: vi.fn(),
      start: vi.fn(),
      stop: vi.fn(),
      buffer: null,
      loop: false,
    })),
    destination: {},
    currentTime: 0,
    sampleRate: 44100,
    resume: vi.fn(),
    state: 'running',
  })),
);

import { create_game_screen } from '../../screens/game';
import { is_multiplayer } from '../../state';
import { set_multiplayer_ws, reset_multiplayer_ws } from '../../core/multiplayer';

vi.mock('../../fx/gameover_fx', () => ({
  run_collapse_animation: vi.fn((_canvas: HTMLCanvasElement, done?: () => void) => {
    done?.();
    return vi.fn();
  }),
}));

vi.mock('../../core/audio', () => ({
  audio_manager: {
    init: vi.fn(async () => {}),
    set_sfx_volume: vi.fn(),
    set_bgm_volume: vi.fn(),
    start_bgm: vi.fn(),
    stop_bgm: vi.fn(),
    play_sfx: vi.fn(),
  },
}));

vi.mock('../../render/board', () => ({
  createBoardRenderer: vi.fn(() => ({ render: vi.fn(), destroy: vi.fn() })),
  create_mini_board_renderer: vi.fn(() => ({ render: vi.fn(), destroy: vi.fn() })),
}));

vi.mock('../../render/preview', () => ({
  createPreviewRenderer: vi.fn(() => ({ render: vi.fn(), destroy: vi.fn() })),
  createNextStackRenderer: vi.fn(() => ({ render: vi.fn(), destroy: vi.fn() })),
}));

vi.mock('../../render/hud', () => ({
  create_hud_overlay: vi.fn(() => ({ update: vi.fn(), destroy: vi.fn() })),
}));

vi.mock('../../input/touch', () => ({
  create_touch_overlay: vi.fn(() => ({ destroy: vi.fn() })),
}));

const raf_callbacks: FrameRequestCallback[] = [];

function run_raf_callbacks(count: number) {
  for (let i = 0; i < count; i++) {
    const cb = raf_callbacks.shift();
    if (!cb) return;
    cb(i * 16.67);
  }
}

function make_mock_ws(send: ReturnType<typeof vi.fn>) {
  return { send, close: vi.fn() };
}

beforeEach(() => {
  is_multiplayer.value = false;
  reset_multiplayer_ws();
  raf_callbacks.length = 0;
  vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb: FrameRequestCallback) => {
    raf_callbacks.push(cb);
    return raf_callbacks.length;
  });
  vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    scale: vi.fn(),
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    fillStyle: '',
  })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
});

describe('create_game_screen', () => {
  it('is a function', () => {
    expect(typeof create_game_screen).toBe('function');
  });

  it('is an async function that accepts root', () => {
    expect(create_game_screen.length).toBeGreaterThanOrEqual(1);
  });

  it('multiplayer batches key input into replay packets', async () => {
    is_multiplayer.value = true;
    const send = vi.fn();
    set_multiplayer_ws(make_mock_ws(send) as never);
    const root = document.createElement('div');

    const destroy = await create_game_screen(root);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    run_raf_callbacks(58);
    expect(send).not.toHaveBeenCalled();

    run_raf_callbacks(2);
    expect(send).toHaveBeenCalledTimes(1);
    expect(send.mock.calls[0][0]).toEqual(new Uint8Array([23, 1]));
    destroy();
  });

  it('single player key input does not send replay packets', async () => {
    is_multiplayer.value = false;
    const send = vi.fn();
    set_multiplayer_ws(make_mock_ws(send) as never);
    const root = document.createElement('div');

    const destroy = await create_game_screen(root);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    run_raf_callbacks(62);

    expect(send).not.toHaveBeenCalled();
    destroy();
  });
});

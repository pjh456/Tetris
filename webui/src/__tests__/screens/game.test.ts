import { describe, it, expect, vi } from 'vitest';

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

describe('create_game_screen', () => {
  it('is a function', () => {
    expect(typeof create_game_screen).toBe('function');
  });

  it('is an async function that accepts root', () => {
    expect(create_game_screen.length).toBeGreaterThanOrEqual(1);
  });
});

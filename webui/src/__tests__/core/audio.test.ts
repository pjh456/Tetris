import { describe, it, expect, vi, beforeEach } from 'vitest';
import { audio_manager } from '../../core/audio';

beforeEach(() => {
  vi.stubGlobal('AudioContext', vi.fn(() => ({
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
  })));
});

describe('audio_manager', () => {
  it('initializes without error', async () => {
    await expect(audio_manager.init()).resolves.not.toThrow();
  });

  it('set_sfx_volume does not throw', () => {
    expect(() => audio_manager.set_sfx_volume(0.5)).not.toThrow();
  });

  it('set_bgm_volume does not throw', () => {
    expect(() => audio_manager.set_bgm_volume(0.3)).not.toThrow();
  });
});

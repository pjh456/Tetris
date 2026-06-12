import { describe, it, expect, vi } from 'vitest';
import { run_collapse_animation } from '../../fx/gameover_fx';

describe('run_collapse_animation', () => {
  it('is a function', () => {
    expect(typeof run_collapse_animation).toBe('function');
  });

  it('accepts canvas and callback without throwing', () => {
    const canvas = document.createElement('canvas');
    const ctx = {
      fillRect: vi.fn(),
      clearRect: vi.fn(),
      fillStyle: '',
      globalAlpha: 1,
      drawImage: vi.fn(),
      getImageData: vi.fn(() => ({ data: new Uint8ClampedArray(4) })),
      putImageData: vi.fn(),
      save: vi.fn(),
      restore: vi.fn(),
    };
    vi.spyOn(canvas, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((cb) => {
      cb(0);
      return 0;
    });
    expect(() => run_collapse_animation(canvas, vi.fn())).not.toThrow();
    vi.restoreAllMocks();
  });
});

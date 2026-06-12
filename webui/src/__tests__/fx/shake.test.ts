import { describe, it, expect, vi } from 'vitest';
import { shake_screen } from '../../fx/shake';

describe('shake_screen', () => {
  it('sets transform on element', () => {
    vi.useFakeTimers();
    const el = document.createElement('div');
    shake_screen(el);
    vi.advanceTimersByTime(33);
    expect(el.style.transform).toContain('translateX');
    vi.useRealTimers();
  });

  it('resets transform after animation completes', () => {
    vi.useFakeTimers();
    const el = document.createElement('div');
    shake_screen(el);
    vi.advanceTimersByTime(33 * 6);
    expect(el.style.transform).toBe('translateX(0px)');
    vi.useRealTimers();
  });
});

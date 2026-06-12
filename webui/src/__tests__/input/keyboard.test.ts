import { describe, it, expect, vi } from 'vitest';
import { bindKeyboard } from '../../input/keyboard';

describe('bindKeyboard', () => {
  it('returns a cleanup function', () => {
    const handlers = {
      handleAction: vi.fn(),
      isGameOver: vi.fn(() => false),
      render: vi.fn(),
      onRelease: vi.fn(),
      onPause: vi.fn(),
    };
    const config = { das_ms: 100, arr_ms: 50 };
    const cleanup = bindKeyboard(handlers, config);
    expect(typeof cleanup).toBe('function');
    cleanup();
  });

  it('calls handleAction on keydown with matching key', () => {
    const handlers = {
      handleAction: vi.fn(),
      isGameOver: vi.fn(() => false),
      render: vi.fn(),
      onRelease: vi.fn(),
      onPause: vi.fn(),
    };
    const config = { das_ms: 100, arr_ms: 50 };
    const cleanup = bindKeyboard(handlers, config);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: ' ' }));
    expect(handlers.handleAction).toHaveBeenCalled();
    cleanup();
  });

  it('cleanup removes event listeners', () => {
    const handlers = {
      handleAction: vi.fn(),
      isGameOver: vi.fn(() => false),
      render: vi.fn(),
      onRelease: vi.fn(),
      onPause: vi.fn(),
    };
    const config = { das_ms: 100, arr_ms: 50 };
    const cleanup = bindKeyboard(handlers, config);
    cleanup();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: ' ' }));
    expect(handlers.handleAction).not.toHaveBeenCalled();
  });
});

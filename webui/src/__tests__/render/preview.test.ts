import { describe, it, expect, vi } from 'vitest';
import { createPreviewRenderer, createNextStackRenderer } from '../../render/preview';

function create_mock_canvas() {
  const canvas = document.createElement('canvas');
  const ctx = {
    fillRect: vi.fn(), clearRect: vi.fn(), fillStyle: '', strokeStyle: '',
    globalAlpha: 1, save: vi.fn(), restore: vi.fn(), scale: vi.fn(),
    strokeRect: vi.fn(), lineWidth: 1, setTransform: vi.fn(),
  };
  vi.spyOn(canvas, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
  return { canvas, ctx };
}

describe('createPreviewRenderer', () => {
  it('returns object with render method', () => {
    const { canvas } = create_mock_canvas();
    const renderer = createPreviewRenderer(canvas);
    expect(typeof renderer.render).toBe('function');
  });
});

describe('createNextStackRenderer', () => {
  it('returns object with render method', () => {
    const { canvas } = create_mock_canvas();
    const renderer = createNextStackRenderer(canvas);
    expect(typeof renderer.render).toBe('function');
  });
});

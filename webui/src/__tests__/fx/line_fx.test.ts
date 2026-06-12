import { describe, it, expect, vi } from 'vitest';
import { LineFx } from '../../fx/line_fx';

function create_mock_canvas() {
  const canvas = document.createElement('canvas');
  canvas.width = 360;
  canvas.height = 720;
  canvas.style.width = '360px';
  const ctx = {
    fillRect: vi.fn(),
    clearRect: vi.fn(),
    fillStyle: '',
    globalAlpha: 1,
    save: vi.fn(),
    restore: vi.fn(),
    strokeRect: vi.fn(),
    strokeStyle: '',
  };
  vi.spyOn(canvas, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
  return { canvas, ctx };
}

describe('LineFx', () => {
  it('constructs without error', () => {
    const { canvas } = create_mock_canvas();
    expect(() => new LineFx(canvas)).not.toThrow();
  });

  it('triggerFlash + update + render calls fillRect', () => {
    const { canvas, ctx } = create_mock_canvas();
    const fx = new LineFx(canvas);
    fx.triggerFlash(360, 36);
    fx.update(100);
    fx.render();
    expect(ctx.fillRect).toHaveBeenCalled();
  });

  it('triggerColumnBurst creates particles', () => {
    const { canvas, ctx } = create_mock_canvas();
    const fx = new LineFx(canvas);
    fx.triggerColumnBurst(0b1111, 0, 720, '#0ff');
    fx.update(16);
    fx.render();
    expect(ctx.fillRect).toHaveBeenCalled();
  });
});

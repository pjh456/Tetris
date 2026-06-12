import { describe, it, expect, vi } from 'vitest';
import { HardDropFx } from '../../fx/harddrop_fx';

function create_mock_canvas() {
  const canvas = document.createElement('canvas');
  canvas.width = 360;
  canvas.height = 720;
  canvas.style.width = '360px';
  const ctx = {
    fillRect: vi.fn(), clearRect: vi.fn(), fillStyle: '', globalAlpha: 1,
    save: vi.fn(), restore: vi.fn(),
  };
  vi.spyOn(canvas, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
  return { canvas, ctx };
}

describe('HardDropFx', () => {
  it('constructs without error', () => {
    const { canvas } = create_mock_canvas();
    expect(() => new HardDropFx(canvas)).not.toThrow();
  });

  it('trigger creates particles that render via fillRect', () => {
    const { canvas, ctx } = create_mock_canvas();
    const fx = new HardDropFx(canvas);
    fx.trigger(0b1111, 0, 720, '#fff');
    fx.update(16);
    fx.render();
    expect(ctx.fillRect).toHaveBeenCalled();
  });

  it('particles decay over time', () => {
    const { canvas, ctx } = create_mock_canvas();
    const fx = new HardDropFx(canvas);
    fx.trigger(0b1, 0, 100, '#fff');
    fx.update(500);
    ctx.fillRect.mockClear();
    fx.render();
    expect(ctx.fillRect.mock.calls.length).toBeLessThanOrEqual(6);
  });
});

import { describe, it, expect, vi } from 'vitest';
import { createBoardRenderer } from '../../render/board';

function create_mock_canvas() {
  const canvas = document.createElement('canvas');
  canvas.width = 360;
  canvas.height = 720;
  canvas.style.width = '360px';
  canvas.style.height = '720px';
  const ctx = {
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    clearRect: vi.fn(),
    fillStyle: '',
    strokeStyle: '',
    globalAlpha: 1,
    lineWidth: 1,
    save: vi.fn(),
    restore: vi.fn(),
    scale: vi.fn(),
    setTransform: vi.fn(),
    beginPath: vi.fn(),
    closePath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    fill: vi.fn(),
    drawImage: vi.fn(),
  };
  vi.spyOn(canvas, 'getContext').mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
  return { canvas, ctx };
}

describe('createBoardRenderer', () => {
  it('returns object with render and destroy methods', () => {
    const { canvas } = create_mock_canvas();
    const renderer = createBoardRenderer(canvas);
    expect(typeof renderer.render).toBe('function');
    expect(typeof renderer.destroy).toBe('function');
  });

  it('render calls fillRect on canvas context', () => {
    const { canvas, ctx } = create_mock_canvas();
    const renderer = createBoardRenderer(canvas);
    const grid = new Uint8Array(200);
    grid[190] = 3;
    const colors = ['#000', '#aaa', '#444', '#0ff', '#ff0', '#f0f', '#0f0', '#f00', '#00f', '#f80'];
    renderer.render(grid, colors);
    expect(ctx.fillRect).toHaveBeenCalled();
  });
});

import { GRID_COLORS } from './colors';

type BoardRenderer = {
  render: (grid: number[]) => void;
};

export function createBoardRenderer(
  canvas: HTMLCanvasElement,
  colors: string[] = GRID_COLORS
): BoardRenderer {
  const ctx = canvas.getContext('2d')!;

  return {
    render(grid: number[]) {
      const cell = Math.min(canvas.width / 10, canvas.height / 20);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (let y = 0; y < 20; y++) {
        for (let x = 0; x < 10; x++) {
          ctx.fillStyle = 'rgba(10, 16, 28, 0.85)';
          ctx.fillRect(x * cell, y * cell, cell, cell);

          ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
          ctx.lineWidth = 1;
          ctx.strokeRect(x * cell + 0.5, y * cell + 0.5, cell - 1, cell - 1);

          const val = grid[y * 10 + x];
          if (val > 0) {
            ctx.fillStyle = colors[val] ?? colors[1];
            ctx.fillRect(x * cell, y * cell, cell - 1, cell - 1);
          }
        }
      }
    }
  };
}

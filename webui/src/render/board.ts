import { GRID_COLORS } from './colors';

type BoardRenderer = {
  render: (grid: number[]) => void;
};

export function createBoardRenderer(
  canvas: HTMLCanvasElement,
  colors: string[] = GRID_COLORS
): BoardRenderer {
  const ctx = canvas.getContext('2d')!;
  const cell = 30;

  return {
    render(grid: number[]) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      for (let y = 0; y < 20; y++) {
        for (let x = 0; x < 10; x++) {
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

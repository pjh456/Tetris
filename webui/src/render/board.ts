const DEFAULT_COLORS = [
  '#000',
  '#aaaaaa',
  '#444444',
  '#00ffff',
  '#ffff00',
  '#ff00ff',
  '#00ff00',
  '#ff0000',
  '#0000ff',
  '#ff8800'
];

type BoardRenderer = {
  render: (grid: number[]) => void;
};

export function createBoardRenderer(
  canvas: HTMLCanvasElement,
  colors: string[] = DEFAULT_COLORS
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
            ctx.fillStyle = colors[val > 3 ? val - 3 : val];
            ctx.fillRect(x * cell, y * cell, cell - 1, cell - 1);
          }
        }
      }
    }
  };
}

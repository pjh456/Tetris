import { get_theme_colors } from './colors';

type BoardRenderer = {
  render: (grid: ArrayLike<number>, colors?: string[]) => void;
  destroy: () => void;
};

function draw_cell(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  x: number,
  y: number,
  val: number,
  colors: string[],
  cell: number,
) {
  ctx.fillStyle = 'rgba(10, 16, 28, 0.85)';
  ctx.fillRect(x * cell, y * cell, cell, cell);

  ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
  ctx.lineWidth = 1;
  ctx.strokeRect(x * cell + 0.5, y * cell + 0.5, cell - 1, cell - 1);

  if (val > 0) {
    ctx.fillStyle = colors[val] ?? colors[1];
    ctx.fillRect(x * cell, y * cell, cell - 1, cell - 1);
  }
}

export function createBoardRenderer(canvas: HTMLCanvasElement): BoardRenderer {
  const ctx = canvas.getContext('2d')!;
  const dpr = window.devicePixelRatio || 1;
  const css_w = parseFloat(canvas.style.width) || canvas.width / dpr;
  const css_h = parseFloat(canvas.style.height) || canvas.height / dpr;
  const cell = Math.min(css_w / 10, css_h / 20);

  const supports_offscreen = typeof OffscreenCanvas !== 'undefined';
  let offscreen: OffscreenCanvas | null = null;
  let off_ctx: OffscreenCanvasRenderingContext2D | null = null;

  if (supports_offscreen) {
    offscreen = new OffscreenCanvas(canvas.width, canvas.height);
    off_ctx = offscreen.getContext('2d')!;
    off_ctx.scale(dpr, dpr);
  }

  const target_ctx = (off_ctx ?? ctx) as
    | CanvasRenderingContext2D
    | OffscreenCanvasRenderingContext2D;
  let prev_grid: Uint8Array | null = null;

  return {
    render(grid: ArrayLike<number>, colors: string[] = get_theme_colors()) {
      const full_redraw = !prev_grid || prev_grid.length !== grid.length;

      if (full_redraw) {
        target_ctx.clearRect(0, 0, css_w, css_h);
        for (let y = 0; y < 20; y++) {
          for (let x = 0; x < 10; x++) {
            draw_cell(target_ctx, x, y, grid[y * 10 + x], colors, cell);
          }
        }
      } else {
        for (let i = 0; i < 200; i++) {
          if (grid[i] !== prev_grid![i]) {
            const x = i % 10;
            const y = Math.floor(i / 10);
            target_ctx.clearRect(x * cell, y * cell, cell, cell);
            draw_cell(target_ctx, x, y, grid[i], colors, cell);
          }
        }
      }

      if (offscreen) {
        ctx.save();
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(offscreen, 0, 0);
        ctx.restore();
      }

      const copy = new Uint8Array(200);
      for (let i = 0; i < 200 && i < grid.length; i++) {
        copy[i] = grid[i];
      }
      prev_grid = copy;
    },

    destroy() {
      prev_grid = null;
      offscreen = null;
      off_ctx = null;
    },
  };
}

export function create_mini_board_renderer(
  canvas: HTMLCanvasElement,
  cell_size: number,
): BoardRenderer {
  const ctx = canvas.getContext('2d')!;
  const w = 10 * cell_size;
  const h = 20 * cell_size;
  canvas.width = w;
  canvas.height = h;
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;

  let prev_grid: Uint8Array | null = null;

  return {
    render(grid: ArrayLike<number>, colors: string[] = get_theme_colors()) {
      for (let i = 0; i < 200 && i < grid.length; i++) {
        const val = grid[i];
        if (prev_grid && val === prev_grid[i]) continue;

        const x = i % 10;
        const y = Math.floor(i / 10);

        ctx.fillStyle = 'rgba(10, 16, 28, 0.85)';
        ctx.fillRect(x * cell_size, y * cell_size, cell_size, cell_size);

        if (val > 0) {
          ctx.fillStyle = colors[val] ?? colors[1];
          ctx.fillRect(x * cell_size, y * cell_size, cell_size - 1, cell_size - 1);
        }
      }

      const copy = new Uint8Array(200);
      for (let i = 0; i < 200 && i < grid.length; i++) {
        copy[i] = grid[i];
      }
      prev_grid = copy;
    },

    destroy() {
      prev_grid = null;
    },
  };
}

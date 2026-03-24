import { GRID_COLORS } from './colors';

const SHAPES: number[][][] = [
  [
    [0, 0, 0, 0],
    [1, 1, 1, 1],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [0, 1, 1, 0],
    [0, 1, 1, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [0, 1, 0, 0],
    [1, 1, 1, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [0, 1, 1, 0],
    [1, 1, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [1, 1, 0, 0],
    [0, 1, 1, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [1, 0, 0, 0],
    [1, 1, 1, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ],
  [
    [0, 0, 1, 0],
    [1, 1, 1, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0]
  ]
];

type PreviewRenderer = {
  render: (pieceId: number) => void;
};

type NextStackRenderer = {
  render: (pieces: number[]) => void;
};

type PreviewOptions = {
  showGrid?: boolean;
};

export function createPreviewRenderer(
  canvas: HTMLCanvasElement,
  options: PreviewOptions = {}
): PreviewRenderer {
  const ctx = canvas.getContext('2d')!;
  const showGrid = options.showGrid !== false;

  return {
    render(pieceId: number) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const cell = canvas.width / 4;
      if (showGrid) {
        for (let y = 0; y < 4; y++) {
          for (let x = 0; x < 4; x++) {
            ctx.fillStyle = 'rgba(10, 16, 28, 0.85)';
            ctx.fillRect(x * cell, y * cell, cell, cell);
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
            ctx.lineWidth = 1;
            ctx.strokeRect(x * cell + 0.5, y * cell + 0.5, cell - 1, cell - 1);
          }
        }
      }

      if (pieceId < 0 || pieceId >= SHAPES.length) return;

      const shape = SHAPES[pieceId];

      ctx.fillStyle = GRID_COLORS[pieceId + 3] ?? GRID_COLORS[3];
      for (let y = 0; y < 4; y++) {
        for (let x = 0; x < 4; x++) {
          if (!shape[y][x]) continue;
          ctx.fillRect(x * cell + 2, y * cell + 2, cell - 4, cell - 4);
        }
      }
    }
  };
}

export function createNextStackRenderer(canvas: HTMLCanvasElement): NextStackRenderer {
  const ctx = canvas.getContext('2d')!;

  return {
    render(pieces: number[]) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const baseCell = canvas.width / 4;
      const pieceHeight = baseCell * 3.2;
      const gap = baseCell * 0.35;

      const total = pieces.length * pieceHeight + (pieces.length - 1) * gap;
      const scale = total > canvas.height ? canvas.height / total : 1;
      const cell = baseCell * scale;
      const offsetX = (canvas.width - 4 * cell) / 2;
      let yCursor = (canvas.height - total * scale) / 2;

      pieces.forEach((pieceId) => {
        if (pieceId >= 0 && pieceId < SHAPES.length) {
          const shape = SHAPES[pieceId];
          ctx.fillStyle = GRID_COLORS[pieceId + 3] ?? GRID_COLORS[3];
          for (let y = 0; y < 4; y++) {
            for (let x = 0; x < 4; x++) {
              if (!shape[y][x]) continue;
              ctx.fillRect(offsetX + x * cell + 2, yCursor + y * cell + 2, cell - 4, cell - 4);
            }
          }
        }
        yCursor += pieceHeight * scale + gap * scale;
      });
    }
  };
}

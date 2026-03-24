const PREVIEW_COLORS = [
  '#00ffff',
  '#ffff00',
  '#ff00ff',
  '#00ff00',
  '#ff0000',
  '#0000ff',
  '#ff8800'
];

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

export function createPreviewRenderer(canvas: HTMLCanvasElement): PreviewRenderer {
  const ctx = canvas.getContext('2d')!;

  return {
    render(pieceId: number) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      if (pieceId < 0 || pieceId >= SHAPES.length) return;

      const shape = SHAPES[pieceId];
      const cell = canvas.width / 4;

      ctx.fillStyle = PREVIEW_COLORS[pieceId];
      for (let y = 0; y < 4; y++) {
        for (let x = 0; x < 4; x++) {
          if (!shape[y][x]) continue;
          ctx.fillRect(x * cell + 2, y * cell + 2, cell - 4, cell - 4);
        }
      }
    }
  };
}

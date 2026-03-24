import createTetrisModule from '@engine/tetris_wasm.js';
import wasmUrl from '@engine/tetris_wasm.wasm?url';
import { bindKeyboard } from './input/keyboard';
import { createBoardRenderer } from './render/board';
import { createPreviewRenderer } from './render/preview';

async function initGame() {
  const wrapper = document.createElement('div');
  wrapper.style.display = 'flex';
  wrapper.style.gap = '16px';
  wrapper.style.alignItems = 'flex-start';
  wrapper.style.justifyContent = 'center';
  wrapper.style.margin = '20px';

  const holdColumn = document.createElement('div');
  holdColumn.style.display = 'flex';
  holdColumn.style.flexDirection = 'column';
  holdColumn.style.gap = '8px';

  const holdLabel = document.createElement('div');
  holdLabel.textContent = 'HOLD';
  holdLabel.style.fontFamily = 'sans-serif';
  holdLabel.style.color = '#333';
  holdLabel.style.fontSize = '14px';

  const holdCanvas = document.createElement('canvas');
  holdCanvas.width = 120;
  holdCanvas.height = 120;
  holdCanvas.style.border = '1px solid #ddd';

  holdColumn.appendChild(holdLabel);
  holdColumn.appendChild(holdCanvas);

  const boardCanvas = document.createElement('canvas');
  boardCanvas.width = 300; // 10列 * 30px
  boardCanvas.height = 600; // 20行 * 30px
  boardCanvas.style.border = '1px solid #ddd';

  const nextColumn = document.createElement('div');
  nextColumn.style.display = 'flex';
  nextColumn.style.flexDirection = 'column';
  nextColumn.style.gap = '8px';

  const nextLabel = document.createElement('div');
  nextLabel.textContent = 'NEXT';
  nextLabel.style.fontFamily = 'sans-serif';
  nextLabel.style.color = '#333';
  nextLabel.style.fontSize = '14px';
  nextColumn.appendChild(nextLabel);

  const nextCanvases: HTMLCanvasElement[] = [];
  for (let i = 0; i < 5; i++) {
    const c = document.createElement('canvas');
    c.width = 120;
    c.height = 120;
    c.style.border = '1px solid #ddd';
    nextColumn.appendChild(c);
    nextCanvases.push(c);
  }

  wrapper.appendChild(holdColumn);
  wrapper.appendChild(boardCanvas);
  wrapper.appendChild(nextColumn);
  document.body.appendChild(wrapper);

  const renderer = createBoardRenderer(boardCanvas);
  const holdRenderer = createPreviewRenderer(holdCanvas);
  const nextRenderers = nextCanvases.map((c) => createPreviewRenderer(c));

  const Module = await createTetrisModule({
    locateFile: (path: string) => {
      if (path.endsWith('.wasm')) {
        return wasmUrl;
      }
      return path;
    }
  });

  const seed = Date.now() >>> 0; // 强制截断为合法的 32 位无符号整数
  const game = new Module.WebTetris(seed);

  const render = () => {
    const grid = game.getGrid();
    renderer.render(grid);
    const hold = game.getHold() as number;
    holdRenderer.render(hold);
    const next = game.getNext() as number[];
    nextRenderers.forEach((r, idx) => r.render(next[idx] ?? -1));
  };

  bindKeyboard({
    handleAction: (action) => game.handleAction(action),
    isGameOver: () => game.isGameOver(),
    render
  });

  let lastTick = performance.now();
  function gameLoop(time: number) {
    if (!game.isGameOver()) {
      if (time - lastTick > 500) {
        game.tick();
        lastTick = time;
        render();
      }
    }
    requestAnimationFrame(gameLoop);
  }

  render();
  requestAnimationFrame(gameLoop);
}

initGame();

import createTetrisModule from '@engine/tetris_wasm.js';
import wasmUrl from '@engine/tetris_wasm.wasm?url';
import { bindKeyboard } from './input/keyboard';
import { createBoardRenderer } from './render/board';
import { createNextStackRenderer, createPreviewRenderer } from './render/preview';

async function initGame() {
  const wrapper = document.createElement('div');
  wrapper.style.display = 'flex';
  wrapper.style.gap = '16px';
  wrapper.style.alignItems = 'flex-start';
  wrapper.style.justifyContent = 'center';
  wrapper.style.margin = '0 auto';
  wrapper.style.height = 'auto';

  document.body.style.margin = '0';
  document.body.style.display = 'flex';
  document.body.style.alignItems = 'center';
  document.body.style.justifyContent = 'center';
  document.body.style.minHeight = '100vh';
  document.body.style.background = '#0b0f1a';

  const holdColumn = document.createElement('div');
  holdColumn.style.display = 'flex';
  holdColumn.style.flexDirection = 'column';
  holdColumn.style.gap = '8px';

  const holdLabel = document.createElement('div');
  holdLabel.textContent = 'HOLD';
  holdLabel.style.fontFamily = 'sans-serif';
  holdLabel.style.color = '#e7f6ff';
  holdLabel.style.fontSize = '14px';
  holdLabel.style.letterSpacing = '2px';

  const holdCanvas = document.createElement('canvas');
  holdCanvas.width = 140;
  holdCanvas.height = 140;
  holdCanvas.style.border = '2px solid rgba(140, 200, 255, 0.9)';
  holdCanvas.style.background = 'rgba(7, 10, 18, 0.9)';
  holdCanvas.style.boxShadow = '0 0 12px rgba(80, 160, 255, 0.35)';

  holdColumn.appendChild(holdLabel);
  holdColumn.appendChild(holdCanvas);

  const boardCanvas = document.createElement('canvas');
  boardCanvas.width = 360; // 10列 * 36px
  boardCanvas.height = 720; // 20行 * 36px
  boardCanvas.style.border = '3px solid rgba(140, 200, 255, 0.95)';
  boardCanvas.style.background = 'rgba(5, 8, 15, 0.95)';
  boardCanvas.style.boxShadow = '0 0 20px rgba(80, 160, 255, 0.45)';

  const nextColumn = document.createElement('div');
  nextColumn.style.display = 'flex';
  nextColumn.style.flexDirection = 'column';
  nextColumn.style.gap = '8px';

  const nextLabel = document.createElement('div');
  nextLabel.textContent = 'NEXT';
  nextLabel.style.fontFamily = 'sans-serif';
  nextLabel.style.background = '#ffffff';
  nextLabel.style.color = '#0b0f1a';
  nextLabel.style.fontSize = '14px';
  nextLabel.style.letterSpacing = '2px';
  nextLabel.style.fontWeight = '600';
  nextLabel.style.textAlign = 'center';
  nextLabel.style.border = '3px solid #ffffff';
  nextLabel.style.padding = '6px 0';
  nextColumn.appendChild(nextLabel);

  const nextCanvas = document.createElement('canvas');
  nextCanvas.width = 180;
  nextCanvas.height = 480; // 约 2/3 棋盘高度
  nextCanvas.style.border = '3px solid rgba(140, 200, 255, 0.95)';
  nextCanvas.style.background = 'rgba(7, 10, 18, 0.9)';
  nextCanvas.style.boxShadow = '0 0 12px rgba(80, 160, 255, 0.35)';
  nextColumn.appendChild(nextCanvas);

  wrapper.appendChild(holdColumn);
  wrapper.appendChild(boardCanvas);
  wrapper.appendChild(nextColumn);
  document.body.appendChild(wrapper);

  const renderer = createBoardRenderer(boardCanvas);
  const holdRenderer = createPreviewRenderer(holdCanvas, { showGrid: true });
  const nextRenderer = createNextStackRenderer(nextCanvas);

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
    nextRenderer.render(next);
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

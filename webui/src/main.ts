import createTetrisModule from '@engine/tetris_wasm.js';
import wasmUrl from '@engine/tetris_wasm.wasm?url';
import { bindKeyboard } from './input/keyboard';
import { createBoardRenderer } from './render/board';

async function initGame() {
  const canvas = document.createElement('canvas');
  canvas.width = 300; // 10列 * 30px
  canvas.height = 600; // 20行 * 30px
  document.body.appendChild(canvas);

  const renderer = createBoardRenderer(canvas);

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

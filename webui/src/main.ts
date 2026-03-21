// 导入 Emscripten 生成的胶水 JS
import createTetrisModule from '@engine/tetris_wasm.js';

// 导入 WASM 二进制文件的静态资源 URL
import wasmUrl from '@engine/tetris_wasm.wasm?url';

const canvas = document.createElement('canvas');
canvas.width = 300; // 10列 * 30px
canvas.height = 600; // 20行 * 30px
document.body.appendChild(canvas);
const ctx = canvas.getContext('2d')!;

// 对应 C++ Engine 中的 Action 枚举
const Actions = {
  MoveLeft: 0, MoveRight: 1, SoftDrop: 2, HardDrop: 3, RotateCW: 4, RotateCCW: 5, Hold: 6
};

// 简单的颜色映射
const colors = ['#000', '#aaaaaa', '#444444', '#00ffff', '#ffff00', '#ff00ff', '#00ff00', '#ff0000', '#0000ff', '#ff8800'];

async function initGame() {
  // 1. 实例化模块时，使用 locateFile 钩子拦截 WASM 文件的请求路径
  const Module = await createTetrisModule({
    locateFile: (path: string) => {
      if (path.endsWith('.wasm')) {
        // 当 Emscripten 请求 .wasm 文件时，返回 Vite 给它的确切 URL
        return wasmUrl;
      }
      return path;
    }
  });

  // 2. 实例化 C++ 游戏类
  const seed = Date.now() >>> 0; // 强制截断为合法的 32 位无符号整数
  const game = new Module.WebTetris(seed);

  // 3. 键盘按键绑定
  window.addEventListener('keydown', (e) => {
    if (game.isGameOver()) return;
    switch (e.key) {
      case 'ArrowLeft': case 'a': game.handleAction(Actions.MoveLeft); break;
      case 'ArrowRight': case 'd': game.handleAction(Actions.MoveRight); break;
      case 'ArrowDown': case 's': game.handleAction(Actions.SoftDrop); break;
      case 'ArrowUp': case 'w': game.handleAction(Actions.RotateCW); break;
      case ' ': game.handleAction(Actions.HardDrop); break;
      case 'c': game.handleAction(Actions.Hold); break;
    }
    render(); // 按键后立即重绘，消除延迟感
  });

  // 4. 定时下落 (tick 循环)
  let lastTick = performance.now();
  function gameLoop(time: number) {
    if (!game.isGameOver()) {
      if (time - lastTick > 500) { // 500ms 自动下落一次
        game.tick();
        lastTick = time;
        render();
      }
    }
    requestAnimationFrame(gameLoop);
  }

  // 5. 渲染函数
  function render() {
    // 从 C++ 内存中读取 10x20 的数组视图
    const grid = game.getGrid();

    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (let y = 0; y < 20; y++) {
      for (let x = 0; x < 10; x++) {
        const val = grid[y * 10 + x];
        if (val > 0) {
          ctx.fillStyle = colors[val > 3 ? val - 3 : val]; // 取决于上面 C++ 代码赋予的值
          ctx.fillRect(x * 30, y * 30, 29, 29); // 留1像素作为网格线
        }
      }
    }
  }

  // 启动游戏
  render();
  requestAnimationFrame(gameLoop);
}

initGame();
import './style.css';

import createTetrisModule from '@engine/tetris_wasm.js';
import wasmUrl from '@engine/tetris_wasm.wasm?url';
import { bindKeyboard } from './input/keyboard';
import { createBoardRenderer } from './render/board';
import { createNextStackRenderer, createPreviewRenderer } from './render/preview';

function createButton(label: string) {
  const btn = document.createElement('button');
  btn.textContent = label;
  btn.className = 'btn';
  return btn;
}

function createHomeScreen(onSingle: () => void, onMulti: () => void) {
  const container = document.createElement('div');
  container.className = 'home';

  const multiCard = document.createElement('div');
  multiCard.className = 'menu-card menu-multi';
  multiCard.onclick = onMulti;
  multiCard.innerHTML = `
    <div class="menu-left">MP</div>
    <div class="menu-body">
      <div class="menu-title">MULTIPLAYER</div>
      <div class="menu-sub">PLAY ONLINE WITH FRIENDS AND FOES</div>
    </div>
  `;

  const soloCard = document.createElement('div');
  soloCard.className = 'menu-card menu-solo';
  soloCard.onclick = onSingle;
  soloCard.innerHTML = `
    <div class="menu-left">SP</div>
    <div class="menu-body">
      <div class="menu-title">SOLO</div>
      <div class="menu-sub">CHALLENGE YOURSELF AND TOP THE LEADERBOARDS</div>
    </div>
  `;

  container.appendChild(multiCard);
  container.appendChild(soloCard);
  return container;
}

function createModal(onClose: () => void, onCreate: () => void, onJoin: () => void) {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay';

  const panel = document.createElement('div');
  panel.className = 'modal';

  const title = document.createElement('div');
  title.textContent = '联机模式';
  title.style.textAlign = 'center';

  const createBtn = createButton('创建房间');
  createBtn.onclick = onCreate;
  const joinBtn = createButton('加入房间');
  joinBtn.onclick = onJoin;
  const closeBtn = createButton('关闭');
  closeBtn.onclick = onClose;

  panel.appendChild(title);
  panel.appendChild(createBtn);
  panel.appendChild(joinBtn);
  panel.appendChild(closeBtn);
  overlay.appendChild(panel);

  return overlay;
}

async function startSingleGame(root: HTMLElement) {
  root.className = 'game-root';
  const wrapper = document.createElement('div');
  wrapper.style.display = 'flex';
  wrapper.style.gap = '16px';
  wrapper.style.alignItems = 'flex-start';
  wrapper.style.justifyContent = 'center';
  wrapper.style.margin = '0 auto';
  wrapper.style.height = 'auto';

  const holdColumn = document.createElement('div');
  holdColumn.style.display = 'flex';
  holdColumn.style.flexDirection = 'column';
  holdColumn.style.gap = '8px';

  const holdLabel = document.createElement('div');
  holdLabel.textContent = 'HOLD';
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
  root.appendChild(wrapper);

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

  const seed = Date.now() >>> 0;
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

function initHome() {
  const app = document.getElementById('app');
  if (!app) return;
  app.innerHTML = '';

  const topbar = document.createElement('div');
  topbar.className = 'topbar';
  topbar.innerHTML = `
    <div class="logo">TETRIS</div>
    <div class="user-info">
      <div>GUEST-6_0160A4</div>
      <div class="status">ANONYMOUS</div>
      <div class="avatar"></div>
    </div>
  `;
  app.appendChild(topbar);

  const root = document.createElement('div');
  root.className = 'content';
  app.appendChild(root);

  const home = createHomeScreen(
    () => {
      root.innerHTML = '';
      root.className = 'content';
      startSingleGame(root);
    },
    () => {
      const modal = createModal(
        () => modal.remove(),
        () => alert('创建房间功能待接入'),
        () => alert('加入房间功能待接入')
      );
      document.body.appendChild(modal);
    }
  );

  root.appendChild(home);
}

initHome();

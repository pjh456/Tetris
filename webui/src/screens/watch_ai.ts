import { WasmAi } from '../../wasm/tetris_wasm.js';
import { create_readonly_board_view } from '../render/readonly_board';

const DECIDE_INTERVAL_MS = 200;

/**
 * WatchAI screen: the local AI plays itself on a read-only board. The viewer
 * cannot interact (no input bindings). On top-out the AI auto-restarts.
 */
export function create_watch_ai_screen(root: HTMLElement): () => void {
  root.innerHTML = '';
  root.className = 'content';

  let ai: WasmAi;
  try {
    ai = new WasmAi(Date.now() >>> 0);
    ai.set_temperature(0);
  } catch {
    root.textContent = 'Failed to load AI.';
    return () => {};
  }

  const wrapper = document.createElement('div');
  wrapper.className = 'watch-ai-root';
  wrapper.style.cssText = 'display:flex;justify-content:center;align-items:center;padding:16px;';

  const board = create_readonly_board_view({
    width: 360,
    height: 720,
    get_grid: () => ai.get_grid(),
  });
  board.mount(wrapper);
  root.appendChild(wrapper);

  let raf_id = 0;
  let last = performance.now();
  let accum = 0;

  function loop(now: number) {
    accum += now - last;
    last = now;
    if (accum >= DECIDE_INTERVAL_MS) {
      accum = 0;
      if (ai.is_game_over()) {
        ai.reset(Date.now() >>> 0);
      } else {
        ai.decide();
      }
    }
    board.render();
    raf_id = requestAnimationFrame(loop);
  }
  raf_id = requestAnimationFrame(loop);

  return () => {
    if (raf_id) {
      cancelAnimationFrame(raf_id);
    }
    board.destroy();
  };
}

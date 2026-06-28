import { WasmAi } from '../../wasm/tetris_wasm.js';
import { create_readonly_board_view } from '../render/readonly_board';
import { createPreviewRenderer, createNextStackRenderer } from '../render/preview';
import { setup_hidpi_canvas } from '../render/canvas';
const ACTION_DELAY_MS = 40;
const SPEED_PRESETS: Record<number, number> = {
  1: 600,
  2: 300,
  3: 150,
  4: 80,
  5: 20,
};

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
  wrapper.style.cssText = 'display:flex;align-items:center;justify-content:center;flex:1;min-height:0;';

  const layout = document.createElement('div');
  layout.className = 'game-layout';
  layout.style.cssText = 'display:flex;gap:16px;align-items:flex-start;justify-content:center;';

  // Hold column
  const hold_col = document.createElement('div');
  hold_col.className = 'hold-panel';
  hold_col.style.cssText = 'display:flex;flex-direction:column;gap:8px;';
  const hold_label = document.createElement('div');
  hold_label.textContent = 'HOLD';
  hold_label.style.cssText =
    'color:var(--color-text);font-size:var(--text-body);letter-spacing:2px;';
  const hold_canvas = document.createElement('canvas');
  setup_hidpi_canvas(hold_canvas, 140, 140);
  hold_canvas.style.border = '2px solid var(--color-panel-border)';
  hold_canvas.style.background = 'var(--color-panel)';
  hold_col.appendChild(hold_label);
  hold_col.appendChild(hold_canvas);

  // Board
  const board_frame = document.createElement('div');
  board_frame.style.cssText =
    'position:relative;display:inline-block;border:3px solid var(--color-panel-border);background:var(--color-bg);box-shadow:0 0 20px var(--color-panel-shadow);';
  const board = create_readonly_board_view({
    width: 360,
    height: 720,
    get_grid: () => ai.get_grid(),
  });
  board.mount(board_frame);

  // Next column
  const right_col = document.createElement('div');
  right_col.className = 'next-panel';
  right_col.style.cssText = 'display:flex;flex-direction:column;gap:8px;';
  const next_label = document.createElement('div');
  next_label.textContent = 'NEXT';
  next_label.style.cssText =
    'color:var(--color-text);font-size:var(--text-body);letter-spacing:2px;text-align:center;background:var(--color-panel);border:1px solid var(--color-panel-border);padding:4px 0;';
  const next_canvas = document.createElement('canvas');
  setup_hidpi_canvas(next_canvas, 180, 480);
  next_canvas.style.border = '1px solid var(--color-panel-border)';
  next_canvas.style.background = 'var(--color-panel)';
  right_col.appendChild(next_label);
  right_col.appendChild(next_canvas);

  // Speed slider
  const slider_row = document.createElement('div');
  slider_row.className = 'watch-ai-speed';
  slider_row.style.cssText =
    'display:flex;align-items:center;gap:12px;padding:8px 0;max-width:360px;margin:0 auto;';
  const speed_label = document.createElement('span');
  speed_label.textContent = 'SPEED';
  speed_label.style.cssText =
    'color:var(--color-muted);font-size:var(--text-label);letter-spacing:2px;white-space:nowrap;';
  const slider = document.createElement('input');
  slider.type = 'range';
  slider.min = '1';
  slider.max = '5';
  slider.value = '3';
  slider.className = 'speed-slider';
  slider.style.cssText = 'flex:1;accent-color:var(--color-accent);';
  const speed_val = document.createElement('span');
  speed_val.textContent = '3';
  speed_val.style.cssText =
    'color:var(--color-accent);font-size:var(--text-body);min-width:20px;text-align:center;font-variant-numeric:tabular-nums;';
  slider_row.appendChild(speed_label);
  slider_row.appendChild(slider);
  slider_row.appendChild(speed_val);

  layout.appendChild(hold_col);
  layout.appendChild(board_frame);
  layout.appendChild(right_col);

  const col_wrap = document.createElement('div');
  col_wrap.style.cssText = 'display:flex;flex-direction:column;align-items:center;gap:12px;';
  col_wrap.appendChild(layout);
  col_wrap.appendChild(slider_row);
  wrapper.appendChild(col_wrap);
  root.appendChild(wrapper);

  const hold_renderer = createPreviewRenderer(hold_canvas, { showGrid: true });
  const next_renderer = createNextStackRenderer(next_canvas);

  let speed = 3;
  let action_queue: number[] = [];
  let action_idx = 0;
  let raf_id = 0;
  let last_action = 0;
  let last_place = 0;

  slider.addEventListener('input', () => {
    speed = parseInt(slider.value, 10);
    speed_val.textContent = String(speed);
  });

  function render_all() {
    board.render();
    hold_renderer.render(ai.get_hold());
    next_renderer.render(Array.from(ai.get_next()));
  }

  function loop(now: number) {
    if (ai.is_game_over()) {
      ai.reset(Date.now() >>> 0);
      action_queue = [];
      action_idx = 0;
      last_place = now;
      last_action = now;
      render_all();
      raf_id = requestAnimationFrame(loop);
      return;
    }

    // Step through action queue (animates piece movement)
    if (action_idx < action_queue.length) {
      if (now - last_action >= ACTION_DELAY_MS) {
        ai.execute_action(action_queue[action_idx++]);
        last_action = now;
        if (action_idx >= action_queue.length) {
          // Piece locked — brief pause before next planning
          last_place = now;
          ai.tick(SPEED_PRESETS[speed] ?? 150);
        }
      }
      render_all();
      raf_id = requestAnimationFrame(loop);
      return;
    }

    // Plan next placement
    const interval = SPEED_PRESETS[speed] ?? 150;
    if (now - last_place >= interval) {
      action_queue = Array.from(ai.decide_plan());
      action_idx = 0;
      last_place = now;
      if (action_queue.length === 0) {
        // No legal placement — tick gravity and retry
        ai.tick(100);
        render_all();
      }
    } else {
      render_all();
    }

    raf_id = requestAnimationFrame(loop);
  }

  last_place = performance.now();
  last_action = performance.now();
  render_all();
  raf_id = requestAnimationFrame(loop);

  return () => {
    if (raf_id) {
      cancelAnimationFrame(raf_id);
    }
    board.destroy();
  };
}

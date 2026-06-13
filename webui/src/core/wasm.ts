import init, { WebTetris, wasm_memory } from '../../wasm/tetris_wasm.js';

interface LoadingElement extends HTMLElement {
  _cleanup?: () => void;
}

let instance: WebTetris | null = null;
let grid_view: Uint8Array | null = null;

function create_grid_view(wasm: WebTetris): Uint8Array {
  const memory = wasm_memory() as unknown as WebAssembly.Memory;
  return new Uint8Array(memory.buffer, wasm.grid_ptr(), wasm.grid_len());
}

export function get_grid_view(): Uint8Array {
  if (!instance) throw new Error('WASM not initialized');
  if (!grid_view || grid_view.byteLength === 0) {
    grid_view = create_grid_view(instance);
  }
  return grid_view;
}

const TIPS = [
  'Press Space to Hard Drop',
  'T-Spin deals double damage',
  'Clear 4 lines for a Tetris!',
  'Hold pieces with Tab',
  'Combos increase attack power',
  'Back-to-Back boosts damage',
];

export function wasm_loading_screen(): HTMLElement {
  const el = document.createElement('div');
  el.style.cssText =
    'display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:24px;';
  el.innerHTML = `
    <div style="font-family:var(--font-display);font-size:48px;color:var(--color-accent);text-shadow:0 0 20px var(--color-accent);letter-spacing:6px;">TETRIS</div>
    <div style="width:200px;height:4px;background:var(--color-panel-border);border-radius:2px;overflow:hidden;">
      <div class="loading-bar" style="height:100%;width:0%;background:var(--color-accent);animation:loading-progress 3s ease-in-out infinite;"></div>
    </div>
    <div class="loading-tip" style="color:var(--color-muted);font-size:14px;min-height:20px;"></div>
  `;

  const style = document.createElement('style');
  style.textContent = `@keyframes loading-progress{0%{width:0%}50%{width:80%}100%{width:100%}}`;
  el.prepend(style);

  const tip_el = el.querySelector('.loading-tip') as HTMLElement;
  let idx = Math.floor(Math.random() * TIPS.length);
  tip_el.textContent = TIPS[idx];
  const timer = setInterval(() => {
    idx = (idx + 1) % TIPS.length;
    tip_el.textContent = TIPS[idx];
  }, 3000);

  (el as LoadingElement)._cleanup = () => clearInterval(timer);
  return el;
}

export function wasm_error_screen(msg: string): HTMLElement {
  const el = document.createElement('div');
  el.style.cssText =
    'display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;gap:16px;';
  el.innerHTML = `
    <div style="font-family:var(--font-display);font-size:26px;color:var(--color-destructive);">Failed to Load</div>
    <div style="color:var(--color-muted);font-size:14px;max-width:400px;text-align:center;">
      Game engine could not be initialized. Check your connection and try again.
    </div>
    <div style="color:var(--color-muted);font-size:12px;opacity:0.6;">${msg}</div>
    <button class="btn" onclick="location.reload()">Retry Loading</button>
  `;
  return el;
}

export async function init_wasm(container?: HTMLElement): Promise<WebTetris> {
  if (instance) return instance;

  const target = container || document.getElementById('app');
  if (!target) throw new Error('No container element');

  const loading = wasm_loading_screen();
  target.innerHTML = '';
  target.appendChild(loading);

  const cleanup_timer = () => {
    (loading as LoadingElement)._cleanup?.();
  };

  try {
    await init();
    const seed = Date.now() >>> 0;
    instance = new WebTetris(seed);
    grid_view = create_grid_view(instance);
    cleanup_timer();
    target.innerHTML = '';
    return instance;
  } catch (err) {
    cleanup_timer();
    target.innerHTML = '';
    target.appendChild(wasm_error_screen(err instanceof Error ? err.message : 'Unknown error'));
    throw err;
  }
}

export function get_wasm(): WebTetris {
  if (!instance) throw new Error('WASM not initialized');
  return instance;
}

export type { WebTetris };

export function reset_wasm(): WebTetris {
  if (!instance) throw new Error('WASM not initialized');
  const seed = Date.now() >>> 0;
  instance.reset(seed);
  grid_view = create_grid_view(instance);
  return instance;
}

export function get_opponent_grid_view(player_id: number): Uint8Array {
  if (!instance) throw new Error('WASM not initialized');
  return instance.get_opponent_grid(player_id);
}

export function get_opponent_count(): number {
  if (!instance) return 0;
  return instance.opponent_count();
}

export function get_opponent_info(index: number): Record<string, unknown> | null {
  if (!instance) return null;
  const result = instance.get_opponent_info(index);
  if (result === null || result === undefined) return null;
  return result as Record<string, unknown>;
}

import init, { WebTetris } from '../../wasm/tetris_wasm.js';

let instance: WebTetris | null = null;

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

  (el as unknown as Record<string, unknown>)._cleanup = () => clearInterval(timer);
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

export async function init_wasm(): Promise<WebTetris> {
  const app = document.getElementById('app');
  if (!app) throw new Error('No #app element');

  const loading = wasm_loading_screen();
  app.innerHTML = '';
  app.appendChild(loading);

  try {
    await init();
    const seed = Date.now() >>> 0;
    instance = new WebTetris(seed);

    const cleanup = (loading as unknown as Record<string, unknown>)._cleanup as
      | (() => void)
      | undefined;
    cleanup?.();
    app.innerHTML = '';
    return instance;
  } catch (err) {
    const cleanup = (loading as unknown as Record<string, unknown>)._cleanup as
      | (() => void)
      | undefined;
    cleanup?.();

    app.innerHTML = '';
    app.appendChild(
      wasm_error_screen(err instanceof Error ? err.message : 'Unknown error'),
    );
    throw err;
  }
}

export function get_wasm(): WebTetris {
  if (!instance) throw new Error('WASM not initialized');
  return instance;
}

export function reset_wasm(): WebTetris {
  if (!instance) throw new Error('WASM not initialized');
  const seed = Date.now() >>> 0;
  instance.reset(seed);
  return instance;
}

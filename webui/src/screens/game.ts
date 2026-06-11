import { init_wasm } from '../core/wasm';
import { page } from '../state';

export async function create_game_screen(root: HTMLElement): Promise<void> {
  root.innerHTML = '';
  root.className = 'content';

  try {
    await init_wasm();
  } catch {
    return;
  }

  const placeholder = document.createElement('div');
  placeholder.className = 'game-root';
  placeholder.innerHTML = `
    <div style="text-align:center;color:var(--color-muted);">
      <div style="font-size:18px;margin-bottom:8px;">GAME</div>
      <div style="font-size:12px;">Full implementation in Plan 07</div>
      <button class="btn" style="margin-top:16px;">Return to Menu</button>
    </div>
  `;
  const btn = placeholder.querySelector('button');
  if (btn) btn.onclick = () => { page.value = 'home'; };
  root.appendChild(placeholder);
}

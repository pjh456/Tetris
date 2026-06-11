import { page } from '../state';

export function create_gameover_screen(): HTMLElement {
  const el = document.createElement('div');
  el.className = 'content';
  el.style.cssText = 'display:flex;align-items:center;justify-content:center;';
  el.innerHTML = `
    <div style="text-align:center;color:var(--color-muted);">
      <div style="font-size:26px;color:var(--color-destructive);margin-bottom:8px;">GAME OVER</div>
      <div style="font-size:12px;">Full implementation in Plan 08</div>
      <button class="btn" style="margin-top:16px;">Return to Menu</button>
    </div>
  `;
  const btn = el.querySelector('button');
  if (btn) btn.onclick = () => { page.value = 'home'; };
  return el;
}

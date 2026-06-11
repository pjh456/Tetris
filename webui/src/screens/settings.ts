import { page } from '../state';

export function create_settings_screen(): HTMLElement {
  const el = document.createElement('div');
  el.className = 'content';
  el.style.cssText = 'display:flex;align-items:center;justify-content:center;';
  el.innerHTML = `
    <div style="text-align:center;color:var(--color-muted);">
      <div style="font-size:18px;margin-bottom:8px;">SETTINGS</div>
      <div style="font-size:12px;">Full implementation in Plan 06</div>
      <button class="btn" style="margin-top:16px;">Return to Menu</button>
    </div>
  `;
  const btn = el.querySelector('button');
  if (btn) btn.onclick = () => { page.value = 'home'; };
  return el;
}

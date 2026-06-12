export type HudData = {
  score: number;
  level: number;
  lines: number;
  combo: number;
  b2b: number;
  tspin: number;
  all_clear: number;
};

function set_text(id: string, text: string) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function flash_element(id: string) {
  const el = document.getElementById(id);
  if (!el) return;
  el.classList.remove('hud-flash');
  void el.offsetWidth;
  el.classList.add('hud-flash');
}

export function create_hud_overlay(container: HTMLElement): {
  update: (data: HudData) => void;
  destroy: () => void;
} {
  const hud = document.createElement('div');
  hud.className = 'hud-panel';
  hud.innerHTML = `
    <div class="hud-item"><span class="hud-label">SCORE</span><span class="hud-value" id="hud-score">0</span></div>
    <div class="hud-item"><span class="hud-label">LEVEL</span><span class="hud-value" id="hud-level">1</span></div>
    <div class="hud-item"><span class="hud-label">LINES</span><span class="hud-value" id="hud-lines">0</span></div>
    <div class="hud-item"><span class="hud-label">COMBO</span><span class="hud-value" id="hud-combo">0</span></div>
    <div class="hud-item" id="hud-b2b-container" style="display:none"><span class="hud-label">B2B</span><span class="hud-value hud-b2b" id="hud-b2b">0</span></div>
  `;
  container.appendChild(hud);

  let prev_score = 0;

  return {
    update(data: HudData) {
      set_text('hud-score', data.score.toLocaleString('en-US'));
      set_text('hud-level', String(data.level));
      set_text('hud-lines', String(data.lines));
      set_text('hud-combo', String(data.combo));

      if (data.score !== prev_score) {
        flash_element('hud-score');
        prev_score = data.score;
      }

      const b2b_el = document.getElementById('hud-b2b-container');
      if (b2b_el) {
        b2b_el.style.display = data.b2b > 0 ? '' : 'none';
        set_text('hud-b2b', `x${data.b2b}`);
      }
    },
    destroy() {
      hud.remove();
    },
  };
}

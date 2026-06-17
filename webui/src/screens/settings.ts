import { page, settings } from '../state';
import {
  load_settings,
  save_settings,
  reset_settings,
  type Settings,
} from '../core/settings_store';
import { apply_theme, type ThemeName } from '../core/theme';

const ACTION_LABELS: Record<string, string> = {
  MoveLeft: 'Move Left',
  MoveRight: 'Move Right',
  SoftDrop: 'Soft Drop',
  HardDrop: 'Hard Drop',
  RotateCW: 'Rotate CW',
  RotateCCW: 'Rotate CCW',
  Hold: 'Hold',
};

const SYSTEM_KEYS = ['F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'];

function key_display(key: string): string {
  if (key === ' ') return 'Space';
  if (key === 'ArrowLeft') return '←';
  if (key === 'ArrowRight') return '→';
  if (key === 'ArrowUp') return '↑';
  if (key === 'ArrowDown') return '↓';
  if (key === 'Tab') return 'Tab';
  if (key.length === 1) return key.toUpperCase();
  return key;
}

function auto_save(s: Settings) {
  save_settings(s);
  settings.value = { ...s };
}

function create_slider(
  label: string,
  value: number,
  min: number,
  max: number,
  step: number,
  unit: string,
  on_change: (v: number) => void,
): HTMLElement {
  const row = document.createElement('div');
  row.className = 'settings-row';
  const lbl = document.createElement('label');
  lbl.textContent = `${label}: ${value}${unit}`;
  const input = document.createElement('input');
  input.type = 'range';
  input.min = String(min);
  input.max = String(max);
  input.step = String(step);
  input.value = String(value);
  input.addEventListener('input', () => {
    const v = Number(input.value);
    lbl.textContent = `${label}: ${v}${unit}`;
    on_change(v);
  });
  row.appendChild(lbl);
  row.appendChild(input);
  return row;
}

export function create_settings_screen(): HTMLElement {
  let current = load_settings();
  const overlay_cleanup: { fn: (() => void) | null } = { fn: null };

  const el = document.createElement('div');
  el.className = 'settings-page glass';

  const header = document.createElement('div');
  header.className = 'settings-header';
  const back_btn = document.createElement('button');
  back_btn.className = 'btn';
  back_btn.textContent = '← Back';
  back_btn.onclick = () => {
    page.value = 'home';
  };
  const title = document.createElement('h2');
  title.className = 'settings-title';
  title.textContent = 'Settings';
  header.appendChild(back_btn);
  header.appendChild(title);
  el.appendChild(header);

  el.appendChild(create_controls_section(current, () => auto_save(current), overlay_cleanup));
  el.appendChild(create_audio_section(current, () => auto_save(current)));
  el.appendChild(create_display_section(current, () => auto_save(current)));

  const reset_btn = document.createElement('button');
  reset_btn.className = 'btn btn-reset';
  reset_btn.textContent = 'Reset to Defaults';
  reset_btn.onclick = () =>
    show_reset_modal(
      el,
      () => {
        current = reset_settings();
        settings.value = { ...current };
        apply_theme(current.theme);
        overlay_cleanup.fn = null;
        page.value = 'settings';
      },
      (cleanup) => {
        overlay_cleanup.fn = cleanup;
      },
    );
  el.appendChild(reset_btn);

  (el as HTMLElement & { _cleanup?: () => void })._cleanup = () => {
    overlay_cleanup.fn?.();
    overlay_cleanup.fn = null;
  };

  return el;
}

function create_controls_section(
  s: Settings,
  on_save: () => void,
  overlay_cleanup: { fn: (() => void) | null },
): HTMLElement {
  const section = document.createElement('section');
  section.className = 'settings-section';
  const h3 = document.createElement('h3');
  h3.textContent = 'Controls';
  section.appendChild(h3);

  section.appendChild(
    create_slider('DAS', s.das_ms, 50, 500, 10, 'ms', (v) => {
      s.das_ms = v;
      on_save();
    }),
  );
  section.appendChild(
    create_slider('ARR', s.arr_ms, 0, 100, 1, 'ms', (v) => {
      s.arr_ms = v;
      on_save();
    }),
  );

  const binds_title = document.createElement('div');
  binds_title.style.cssText =
    'font-size:var(--text-label);color:var(--color-muted);margin:var(--space-md) 0 var(--space-xs);';
  binds_title.textContent = 'KEY BINDINGS';
  section.appendChild(binds_title);

  for (const action of Object.keys(ACTION_LABELS)) {
    const bind = s.keymap[action];
    if (!bind) continue;

    const row = document.createElement('div');
    row.className = 'key-bind-row';

    const name_el = document.createElement('span');
    name_el.textContent = ACTION_LABELS[action];

    const key_el = document.createElement('span');
    key_el.className = 'key-bind-label';
    key_el.textContent = key_display(bind.key);
    key_el.onclick = () => {
      show_rebind_overlay(
        action,
        (new_key, new_code) => {
          s.keymap[action] = { key: new_key, code: new_code };
          key_el.textContent = key_display(new_key);
          overlay_cleanup.fn = null;
          on_save();
        },
        (cleanup) => {
          overlay_cleanup.fn = cleanup;
        },
      );
    };

    row.appendChild(name_el);
    row.appendChild(key_el);
    section.appendChild(row);
  }

  return section;
}

function create_audio_section(s: Settings, on_save: () => void): HTMLElement {
  const section = document.createElement('section');
  section.className = 'settings-section';
  const h3 = document.createElement('h3');
  h3.textContent = 'Audio';
  section.appendChild(h3);

  section.appendChild(
    create_slider('SFX', Math.round(s.sfx_volume * 100), 0, 100, 1, '%', (v) => {
      s.sfx_volume = v / 100;
      on_save();
    }),
  );
  section.appendChild(
    create_slider('BGM', Math.round(s.bgm_volume * 100), 0, 100, 1, '%', (v) => {
      s.bgm_volume = v / 100;
      on_save();
    }),
  );

  return section;
}

function create_display_section(s: Settings, on_save: () => void): HTMLElement {
  const section = document.createElement('section');
  section.className = 'settings-section';
  const h3 = document.createElement('h3');
  h3.textContent = 'Display';
  section.appendChild(h3);

  const theme_row = document.createElement('div');
  theme_row.className = 'settings-row';
  const theme_label = document.createElement('label');
  theme_label.textContent = 'Theme';
  theme_row.appendChild(theme_label);

  const theme_group = document.createElement('div');
  theme_group.style.cssText = 'display:flex;gap:var(--space-sm);';
  for (const t of ['cyberpunk', 'retro', 'minimal'] as const) {
    const btn = document.createElement('button');
    btn.className = 'btn';
    btn.textContent = t.charAt(0).toUpperCase() + t.slice(1);
    if (s.theme === t) btn.style.borderColor = 'var(--color-accent)';
    btn.onclick = () => {
      s.theme = t;
      apply_theme(t as ThemeName);
      on_save();
      for (const child of theme_group.children) {
        (child as HTMLElement).style.borderColor = '';
      }
      btn.style.borderColor = 'var(--color-accent)';
    };
    theme_group.appendChild(btn);
  }
  theme_row.appendChild(theme_group);
  section.appendChild(theme_row);

  const countdown_row = document.createElement('div');
  countdown_row.className = 'settings-row';
  const cd_label = document.createElement('label');
  cd_label.textContent = '3-2-1-GO Countdown';
  const cd_check = document.createElement('input');
  cd_check.type = 'checkbox';
  cd_check.checked = s.show_countdown;
  cd_check.addEventListener('change', () => {
    s.show_countdown = cd_check.checked;
    on_save();
  });
  countdown_row.appendChild(cd_label);
  countdown_row.appendChild(cd_check);
  section.appendChild(countdown_row);

  return section;
}

function show_rebind_overlay(
  action: string,
  on_bind: (key: string, code: string) => void,
  store_cleanup: (cleanup: () => void) => void,
) {
  const overlay = document.createElement('div');
  overlay.className = 'key-rebind-overlay';
  overlay.innerHTML = `
    <div class="key-rebind-prompt glass">
      <div style="font-size:var(--text-heading);margin-bottom:var(--space-md);">Press a key for ${ACTION_LABELS[action]}</div>
      <div style="color:var(--color-muted);font-size:var(--text-body);">Press Escape to cancel</div>
    </div>
  `;

  const handler = (e: KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      cleanup();
      return;
    }

    if (SYSTEM_KEYS.includes(e.key) || e.ctrlKey || e.altKey || e.metaKey) {
      return;
    }

    on_bind(e.key, e.code);
    cleanup();
  };

  const cleanup = () => {
    document.removeEventListener('keydown', handler, true);
    overlay.remove();
  };
  store_cleanup(cleanup);

  document.addEventListener('keydown', handler, true);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) cleanup();
  });
  document.body.appendChild(overlay);
}

function show_reset_modal(
  _parent: HTMLElement,
  on_reset: () => void,
  store_cleanup: (cleanup: () => void) => void,
) {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay';
  overlay.innerHTML = `
    <div class="modal glass">
      <div style="text-align:center;font-size:var(--text-body);">
        Reset all settings to their default values?<br>This cannot be undone.
      </div>
      <div style="display:flex;gap:var(--space-sm);justify-content:center;">
        <button class="btn btn-reset" id="confirm-reset">Reset</button>
        <button class="btn" id="cancel-reset">Keep Settings</button>
      </div>
    </div>
  `;

  overlay.querySelector('#confirm-reset')!.addEventListener('click', () => {
    overlay.remove();
    on_reset();
  });
  overlay.querySelector('#cancel-reset')!.addEventListener('click', () => {
    overlay.remove();
  });
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) overlay.remove();
  });

  store_cleanup(() => overlay.remove());
  document.body.appendChild(overlay);
}

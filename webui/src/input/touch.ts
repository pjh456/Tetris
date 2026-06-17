import { settings } from '../state';

interface TouchButton {
  x: number;
  y: number;
  w: number;
  h: number;
  action: number;
  symbol: string;
  key: string;
  el: HTMLElement;
  held_start: number;
}

const BUTTON_DEFS = [
  { action: 0, symbol: '\u{25C0}', key: 'MoveLeft' },
  { action: 2, symbol: '\u{25BC}', key: 'SoftDrop' },
  { action: 1, symbol: '\u{25B6}', key: 'MoveRight' },
  { action: 4, symbol: '\u{21BB}', key: 'RotateCW' },
  { action: 3, symbol: '\u{23EC}', key: 'HardDrop' },
  { action: 5, symbol: '\u{21BA}', key: 'RotateCCW' },
  { action: 6, symbol: '\u{23CF}', key: 'Hold' },
];

const DAS_REPEAT_ACTIONS = new Set([0, 1, 2]);

function place_btn(btn: TouchButton, x: number, y: number, size: number) {
  btn.x = x;
  btn.y = y;
  btn.w = size;
  btn.h = size;
  btn.el.style.left = `${x}px`;
  btn.el.style.top = `${y}px`;
  btn.el.style.width = `${size}px`;
  btn.el.style.height = `${size}px`;
}

export function create_touch_overlay(
  container: HTMLElement,
  on_action: (action: number) => void,
): { destroy: () => void } {
  const overlay = document.createElement('div');
  overlay.className = 'touch-overlay';
  container.appendChild(overlay);

  const buttons: TouchButton[] = [];
  const active_touches = new Map<number, TouchButton>();
  let das_timer: number | null = null;

  for (const def of BUTTON_DEFS) {
    const el = document.createElement('div');
    el.className = 'touch-btn';
    el.textContent = def.symbol;
    el.dataset.key = def.key;
    overlay.appendChild(el);

    buttons.push({
      x: 0,
      y: 0,
      w: 72,
      h: 72,
      action: def.action,
      symbol: def.symbol,
      key: def.key,
      el,
      held_start: 0,
    });
  }

  function layout_buttons() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    const btn_size = Math.min(w, h) < 480 ? 56 : 72;
    const gap = 12;
    const is_portrait = h > w;

    if (is_portrait) {
      const dpad_left = (w - btn_size * 5 - gap * 2) / 2;
      const bottom_y = h - btn_size * 3 - gap * 2 - 20;
      const act_left = w / 2 + gap;

      place_btn(buttons[0], dpad_left, bottom_y + btn_size, btn_size);
      place_btn(buttons[1], dpad_left + btn_size + gap, bottom_y + btn_size * 2, btn_size);
      place_btn(buttons[2], dpad_left + (btn_size + gap) * 2, bottom_y + btn_size, btn_size);
      place_btn(buttons[3], act_left, bottom_y, btn_size);
      place_btn(buttons[4], act_left + btn_size + gap, bottom_y + btn_size, btn_size * 2 + gap);
      place_btn(buttons[5], act_left + btn_size + gap, bottom_y, btn_size);
      place_btn(buttons[6], act_left, bottom_y + btn_size + gap, btn_size);
    } else {
      const left_x = 20;
      const right_x = w - btn_size * 3 - gap - 20;
      const center_y = (h - btn_size * 2 - gap) / 2;

      place_btn(buttons[0], left_x, center_y + btn_size, btn_size);
      place_btn(buttons[1], left_x + btn_size + gap, center_y + btn_size * 2, btn_size);
      place_btn(buttons[2], left_x + (btn_size + gap) * 2, center_y + btn_size, btn_size);
      place_btn(buttons[3], right_x, center_y, btn_size);
      place_btn(buttons[4], right_x + btn_size + gap, center_y + btn_size, btn_size * 2 + gap);
      place_btn(buttons[5], right_x + btn_size + gap, center_y, btn_size);
      place_btn(buttons[6], right_x, center_y + btn_size + gap, btn_size);
    }
  }

  function hit_test(tx: number, ty: number): TouchButton | null {
    for (const btn of buttons) {
      if (tx >= btn.x && tx <= btn.x + btn.w && ty >= btn.y && ty <= btn.y + btn.h) {
        return btn;
      }
    }
    return null;
  }

  function vibrate(pattern: number | number[]) {
    if (navigator.vibrate) {
      navigator.vibrate(pattern);
    }
  }

  overlay.addEventListener('touchstart', (e) => {
    e.preventDefault();
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      const btn = hit_test(touch.clientX, touch.clientY);
      if (btn) {
        active_touches.set(touch.identifier, btn);
        btn.el.classList.add('active');
        btn.held_start = performance.now();
        on_action(btn.action);
        vibrate(10);
      }
    }
  });

  overlay.addEventListener('touchmove', (e) => {
    e.preventDefault();
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      const btn = active_touches.get(touch.identifier);
      if (btn && !hit_test(touch.clientX, touch.clientY)) {
        btn.el.classList.remove('active');
        active_touches.delete(touch.identifier);
      }
    }
  });

  overlay.addEventListener('touchend', (e) => {
    e.preventDefault();
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      const btn = active_touches.get(touch.identifier);
      if (btn) {
        btn.el.classList.remove('active');
        active_touches.delete(touch.identifier);
      }
    }
  });

  overlay.addEventListener('touchcancel', (e) => {
    e.preventDefault();
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i];
      const btn = active_touches.get(touch.identifier);
      if (btn) {
        btn.el.classList.remove('active');
        active_touches.delete(touch.identifier);
      }
    }
  });

  das_timer = window.setInterval(() => {
    const now = performance.now();
    const das = settings.value.das_ms;
    const arr = Math.max(settings.value.arr_ms, 1);
    for (const [, btn] of active_touches) {
      const held = now - btn.held_start;
      if (held >= das) {
        const arr_elapsed = held - das;
        const tick_count = Math.floor(arr_elapsed / arr);
        if (tick_count > 0) {
          btn.held_start = now - das;
          if (DAS_REPEAT_ACTIONS.has(btn.action)) {
            on_action(btn.action);
          }
        }
      }
    }
  }, 16);

  layout_buttons();
  window.addEventListener('resize', layout_buttons);
  const orientation_handler = () => {
    setTimeout(layout_buttons, 100);
  };
  window.addEventListener('orientationchange', orientation_handler);

  return {
    destroy() {
      window.removeEventListener('resize', layout_buttons);
      window.removeEventListener('orientationchange', orientation_handler);
      if (das_timer) clearInterval(das_timer);
      overlay.remove();
    },
  };
}

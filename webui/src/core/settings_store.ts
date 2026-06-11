export type Settings = {
  das_ms: number;
  arr_ms: number;
  sfx_volume: number;
  bgm_volume: number;
  theme: 'cyberpunk' | 'retro' | 'minimal';
  show_countdown: boolean;
};

const DEFAULTS: Settings = {
  das_ms: 133,
  arr_ms: 10,
  sfx_volume: 0.8,
  bgm_volume: 0.5,
  theme: 'cyberpunk',
  show_countdown: true,
};

const STORAGE_KEY = 'tetris-settings';
const VALID_THEMES = ['cyberpunk', 'retro', 'minimal'];

function validate_number(v: unknown, fallback: number, min: number, max: number): number {
  if (typeof v !== 'number' || isNaN(v)) return fallback;
  return Math.max(min, Math.min(max, v));
}

function validate_theme(v: unknown): Settings['theme'] {
  if (typeof v === 'string' && VALID_THEMES.includes(v)) return v as Settings['theme'];
  return DEFAULTS.theme;
}

export function load_settings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw);
    return {
      das_ms: validate_number(parsed.das_ms, DEFAULTS.das_ms, 50, 500),
      arr_ms: validate_number(parsed.arr_ms, DEFAULTS.arr_ms, 0, 100),
      sfx_volume: validate_number(parsed.sfx_volume, DEFAULTS.sfx_volume, 0, 1),
      bgm_volume: validate_number(parsed.bgm_volume, DEFAULTS.bgm_volume, 0, 1),
      theme: validate_theme(parsed.theme),
      show_countdown:
        typeof parsed.show_countdown === 'boolean' ? parsed.show_countdown : DEFAULTS.show_countdown,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function save_settings(s: Settings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
}

export function reset_settings(): Settings {
  const defaults = { ...DEFAULTS };
  save_settings(defaults);
  return defaults;
}

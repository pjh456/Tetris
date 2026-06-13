export type KeyBind = { key: string; code: string };

export type Settings = {
  das_ms: number;
  arr_ms: number;
  sfx_volume: number;
  bgm_volume: number;
  theme: 'cyberpunk' | 'retro' | 'minimal';
  show_countdown: boolean;
  keymap: Record<string, KeyBind>;
};

const DEFAULT_KEYMAP: Record<string, KeyBind> = {
  MoveLeft: { key: 'ArrowLeft', code: 'ArrowLeft' },
  MoveRight: { key: 'ArrowRight', code: 'ArrowRight' },
  SoftDrop: { key: 'ArrowDown', code: 'ArrowDown' },
  HardDrop: { key: ' ', code: 'Space' },
  RotateCW: { key: 'ArrowUp', code: 'ArrowUp' },
  RotateCCW: { key: 'z', code: 'KeyZ' },
  Hold: { key: 'Tab', code: 'Tab' },
};

const DEFAULTS: Settings = {
  das_ms: 100,
  arr_ms: 50,
  sfx_volume: 0.8,
  bgm_volume: 0.5,
  theme: 'cyberpunk',
  show_countdown: true,
  keymap: { ...DEFAULT_KEYMAP },
};

const STORAGE_KEY = 'tetris-settings';
const VALID_THEMES = ['cyberpunk', 'retro', 'minimal'];
const ACTIONS = ['MoveLeft', 'MoveRight', 'SoftDrop', 'HardDrop', 'RotateCW', 'RotateCCW', 'Hold'];

function validate_number(v: unknown, fallback: number, min: number, max: number): number {
  if (typeof v !== 'number' || isNaN(v)) return fallback;
  return Math.max(min, Math.min(max, v));
}

function validate_theme(v: unknown): Settings['theme'] {
  if (typeof v === 'string' && VALID_THEMES.includes(v)) return v as Settings['theme'];
  return DEFAULTS.theme;
}

function validate_keymap(v: unknown): Record<string, KeyBind> {
  const result = { ...DEFAULT_KEYMAP };
  if (typeof v !== 'object' || v === null) return result;
  const obj = v as Record<string, unknown>;
  for (const action of ACTIONS) {
    const bind = obj[action];
    if (
      bind &&
      typeof bind === 'object' &&
      typeof (bind as KeyBind).key === 'string' &&
      typeof (bind as KeyBind).code === 'string'
    ) {
      result[action] = { key: (bind as KeyBind).key, code: (bind as KeyBind).code };
    }
  }
  return result;
}

export function load_settings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS, keymap: { ...DEFAULT_KEYMAP } };
    const parsed = JSON.parse(raw);
    return {
      das_ms: validate_number(parsed.das_ms, DEFAULTS.das_ms, 50, 500),
      arr_ms: validate_number(parsed.arr_ms, DEFAULTS.arr_ms, 0, 100),
      sfx_volume: validate_number(parsed.sfx_volume, DEFAULTS.sfx_volume, 0, 1),
      bgm_volume: validate_number(parsed.bgm_volume, DEFAULTS.bgm_volume, 0, 1),
      theme: validate_theme(parsed.theme),
      show_countdown:
        typeof parsed.show_countdown === 'boolean'
          ? parsed.show_countdown
          : DEFAULTS.show_countdown,
      keymap: validate_keymap(parsed.keymap),
    };
  } catch {
    return { ...DEFAULTS, keymap: { ...DEFAULT_KEYMAP } };
  }
}

export function save_settings(s: Settings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
}

export function reset_settings(): Settings {
  const defaults = { ...DEFAULTS, keymap: { ...DEFAULT_KEYMAP } };
  save_settings(defaults);
  return defaults;
}

export type LeaderboardEntry = {
  score: number;
  level: number;
  lines: number;
  date: string;
};

const LEADERBOARD_KEY = 'tetris-leaderboard';

export function get_leaderboard(): LeaderboardEntry[] {
  try {
    const raw = localStorage.getItem(LEADERBOARD_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (e: unknown) =>
          e &&
          typeof e === 'object' &&
          typeof (e as LeaderboardEntry).score === 'number' &&
          typeof (e as LeaderboardEntry).level === 'number' &&
          typeof (e as LeaderboardEntry).lines === 'number' &&
          typeof (e as LeaderboardEntry).date === 'string',
      )
      .sort((a: LeaderboardEntry, b: LeaderboardEntry) => b.score - a.score)
      .slice(0, 10);
  } catch {
    return [];
  }
}

export function save_score_to_leaderboard(score: number, level: number, lines: number): number {
  const entries = get_leaderboard();
  const entry: LeaderboardEntry = {
    score,
    level,
    lines,
    date: new Date().toISOString(),
  };
  entries.push(entry);
  entries.sort((a, b) => b.score - a.score);
  const top10 = entries.slice(0, 10);
  localStorage.setItem(LEADERBOARD_KEY, JSON.stringify(top10));
  const rank = top10.findIndex((e) => e === entry);
  return rank >= 0 ? rank + 1 : 0;
}

export function clear_leaderboard(): void {
  localStorage.removeItem(LEADERBOARD_KEY);
}

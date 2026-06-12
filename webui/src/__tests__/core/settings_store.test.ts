import { describe, it, expect, beforeEach } from 'vitest';
import { load_settings, save_settings, reset_settings } from '../../core/settings_store';

beforeEach(() => {
  localStorage.clear();
});

describe('load_settings', () => {
  it('returns defaults when localStorage is empty', () => {
    const s = load_settings();
    expect(s.das_ms).toBe(100);
    expect(s.arr_ms).toBe(50);
    expect(s.sfx_volume).toBe(0.8);
    expect(s.bgm_volume).toBe(0.5);
    expect(s.theme).toBe('cyberpunk');
    expect(s.show_countdown).toBe(true);
  });

  it('falls back to defaults on corrupted JSON', () => {
    localStorage.setItem('tetris-settings', '{broken json!!!');
    const s = load_settings();
    expect(s.das_ms).toBe(100);
    expect(s.theme).toBe('cyberpunk');
  });

  it('clamps numeric values to valid range', () => {
    localStorage.setItem(
      'tetris-settings',
      JSON.stringify({ das_ms: 9999, arr_ms: -10, sfx_volume: 5 }),
    );
    const s = load_settings();
    expect(s.das_ms).toBe(500);
    expect(s.arr_ms).toBe(0);
    expect(s.sfx_volume).toBe(1);
  });

  it('validates theme to known values', () => {
    localStorage.setItem('tetris-settings', JSON.stringify({ theme: 'invalid' }));
    const s = load_settings();
    expect(s.theme).toBe('cyberpunk');
  });

  it('preserves valid custom keymap', () => {
    const custom = { MoveLeft: { key: 'a', code: 'KeyA' } };
    localStorage.setItem('tetris-settings', JSON.stringify({ keymap: custom }));
    const s = load_settings();
    expect(s.keymap.MoveLeft.key).toBe('a');
    expect(s.keymap.MoveRight.key).toBe('ArrowRight');
  });
});

describe('save_settings + load_settings roundtrip', () => {
  it('persists and reads back identical settings', () => {
    const s = load_settings();
    s.theme = 'retro';
    s.das_ms = 133;
    save_settings(s);
    const loaded = load_settings();
    expect(loaded.theme).toBe('retro');
    expect(loaded.das_ms).toBe(133);
  });
});

describe('reset_settings', () => {
  it('restores defaults and persists them', () => {
    save_settings({ ...load_settings(), theme: 'minimal' });
    const s = reset_settings();
    expect(s.theme).toBe('cyberpunk');
    const loaded = load_settings();
    expect(loaded.theme).toBe('cyberpunk');
  });
});

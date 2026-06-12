import { describe, it, expect, beforeEach } from 'vitest';
import { apply_theme, get_current_theme } from '../../core/theme';

beforeEach(() => {
  document.body.removeAttribute('data-theme');
});

describe('apply_theme', () => {
  it('sets data-theme attribute on body', () => {
    apply_theme('retro');
    expect(document.body.getAttribute('data-theme')).toBe('retro');
  });

  it('falls back to cyberpunk for invalid theme', () => {
    apply_theme('invalid' as 'cyberpunk');
    expect(document.body.getAttribute('data-theme')).toBe('cyberpunk');
  });
});

describe('get_current_theme', () => {
  it('returns cyberpunk when no theme is set', () => {
    expect(get_current_theme()).toBe('cyberpunk');
  });

  it('returns the set theme', () => {
    apply_theme('minimal');
    expect(get_current_theme()).toBe('minimal');
  });
});

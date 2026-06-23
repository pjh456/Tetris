import { describe, it, expect } from 'vitest';
import { get_theme_colors } from '../../render/colors';

describe('get_theme_colors', () => {
  it('returns array of 12 colors (10 cell values + garbage center/border)', () => {
    const colors = get_theme_colors();
    expect(colors).toHaveLength(12);
    expect(colors[10]).toBeTruthy();
    expect(colors[11]).toBeTruthy();
  });

  it('first color is black (empty cell)', () => {
    const colors = get_theme_colors();
    expect(colors[0]).toBe('#000000');
  });

  it('provides fallback colors when CSS vars are missing', () => {
    const colors = get_theme_colors();
    colors.forEach((c) => {
      expect(c).toBeTruthy();
    });
  });
});

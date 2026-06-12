import { describe, it, expect } from 'vitest';
import { create_hud_overlay } from '../../render/hud';

describe('create_hud_overlay', () => {
  it('returns object with update and destroy methods', () => {
    const container = document.createElement('div');
    const hud = create_hud_overlay(container);
    expect(typeof hud.update).toBe('function');
    expect(typeof hud.destroy).toBe('function');
  });

  it('update does not throw with valid data', () => {
    const container = document.createElement('div');
    const hud = create_hud_overlay(container);
    expect(() =>
      hud.update({
        score: 1000,
        level: 2,
        lines: 10,
        combo: 3,
        b2b: 1,
        tspin: 0,
        all_clear: 0,
      }),
    ).not.toThrow();
  });

  it('destroy removes HUD from DOM', () => {
    const container = document.createElement('div');
    const hud = create_hud_overlay(container);
    hud.destroy();
    expect(container.querySelector('.hud-panel')).toBeNull();
  });
});

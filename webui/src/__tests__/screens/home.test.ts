import { describe, it, expect, vi } from 'vitest';
import { create_home_screen } from '../../screens/home';

describe('create_home_screen', () => {
  it('returns an HTML element', () => {
    const el = create_home_screen(vi.fn(), vi.fn(), vi.fn());
    expect(el).toBeInstanceOf(HTMLElement);
  });

  it('contains SOLO text', () => {
    const el = create_home_screen(vi.fn(), vi.fn(), vi.fn());
    expect(el.textContent).toContain('SOLO');
  });

  it('contains SETTINGS text', () => {
    const el = create_home_screen(vi.fn(), vi.fn(), vi.fn());
    expect(el.textContent).toContain('SETTINGS');
  });
});

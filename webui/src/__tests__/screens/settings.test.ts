import { describe, it, expect } from 'vitest';
import { create_settings_screen } from '../../screens/settings';

describe('create_settings_screen', () => {
  it('returns an HTML element', () => {
    const el = create_settings_screen();
    expect(el).toBeInstanceOf(HTMLElement);
  });

  it('contains Controls section', () => {
    const el = create_settings_screen();
    expect(el.textContent).toContain('Controls');
  });

  it('contains Audio section', () => {
    const el = create_settings_screen();
    expect(el.textContent).toContain('Audio');
  });
});

import { describe, it, expect, vi } from 'vitest';

vi.mock('../../../wasm/tetris_wasm.js', () => import('../../__mocks__/tetris_wasm.js'));

import { create_gameover_screen } from '../../screens/gameover';

describe('create_gameover_screen', () => {
  it('returns an HTML element', () => {
    const el = create_gameover_screen();
    expect(el).toBeInstanceOf(HTMLElement);
  });

  it('displays GAME OVER title', () => {
    const el = create_gameover_screen();
    expect(el.textContent).toContain('GAME OVER');
  });

  it('contains retry and menu buttons', () => {
    const el = create_gameover_screen();
    expect(el.querySelector('#go-retry')).not.toBeNull();
    expect(el.querySelector('#go-menu')).not.toBeNull();
  });
});

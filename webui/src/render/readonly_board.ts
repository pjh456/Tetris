import { createBoardRenderer } from './board';
import { setup_hidpi_canvas } from './canvas';

export interface ReadonlyBoardView {
  mount: (parent: HTMLElement) => void;
  render: () => void;
  destroy: () => void;
}

/**
 * Read-only board view: renders a grid from `get_grid()` with no input bindings.
 * Used by the WatchAI screen and reused by multiplayer spectating (PLAN-25).
 */
export function create_readonly_board_view(opts: {
  width: number;
  height: number;
  get_grid: () => Uint8Array | null;
}): ReadonlyBoardView {
  const canvas = document.createElement('canvas');
  setup_hidpi_canvas(canvas, opts.width, opts.height);
  const renderer = createBoardRenderer(canvas);

  return {
    mount(parent: HTMLElement) {
      parent.appendChild(canvas);
    },
    render() {
      const grid = opts.get_grid();
      if (grid) {
        renderer.render(grid);
      }
    },
    destroy() {
      renderer.destroy();
      canvas.remove();
    },
  };
}

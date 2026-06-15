import { Actions, type ActionValue } from '../game/actions';

type KeyboardTargets = {
  handleAction: (action: ActionValue) => void;
  isGameOver: () => boolean;
  render: () => void;
  onPress?: (action: ActionValue) => void;
  onRelease?: (action: ActionValue) => void;
  onPause?: () => void;
};

export type KeyboardConfig = {
  das_ms: number;
  arr_ms: number;
  keymap?: Record<string, { key: string; code: string }>;
};

const DEFAULT_KEY_MAP: Record<string, ActionValue> = {
  ArrowLeft: Actions.MoveLeft,
  a: Actions.MoveLeft,
  ArrowRight: Actions.MoveRight,
  d: Actions.MoveRight,
  ArrowDown: Actions.SoftDrop,
  s: Actions.SoftDrop,
  ArrowUp: Actions.RotateCW,
  w: Actions.RotateCW,
  x: Actions.RotateCW,
  z: Actions.RotateCCW,
  ' ': Actions.HardDrop,
  Tab: Actions.Hold,
  c: Actions.Hold,
};

const ACTION_NAME_MAP: Record<string, ActionValue> = {
  MoveLeft: Actions.MoveLeft,
  MoveRight: Actions.MoveRight,
  SoftDrop: Actions.SoftDrop,
  HardDrop: Actions.HardDrop,
  RotateCW: Actions.RotateCW,
  RotateCCW: Actions.RotateCCW,
  Hold: Actions.Hold,
};

function build_key_map(
  keymap?: Record<string, { key: string; code: string }>,
): Record<string, ActionValue> {
  if (!keymap) return DEFAULT_KEY_MAP;
  const result: Record<string, ActionValue> = {};
  for (const [action_name, bind] of Object.entries(keymap)) {
    const action = ACTION_NAME_MAP[action_name];
    if (action !== undefined) {
      result[bind.key] = action;
    }
  }
  return result;
}

const REPEAT_ACTIONS = new Set<ActionValue>([
  Actions.MoveLeft,
  Actions.MoveRight,
  Actions.SoftDrop,
]);

const BASE_PREVENT_KEYS = new Set([
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  ' ',
  'z',
  'x',
  'c',
  'Tab',
  'Escape',
]);

export function bindKeyboard(
  targets: KeyboardTargets,
  config: KeyboardConfig = { das_ms: 100, arr_ms: 50 },
): () => void {
  const { handleAction, isGameOver, render } = targets;
  const key_map = build_key_map(config.keymap);
  const prevent_keys = new Set([...BASE_PREVENT_KEYS, ...Object.keys(key_map)]);

  type HoldState = { pressedAt: number; lastFire: number };
  const held = new Map<ActionValue, HoldState>();
  const one_shot_active = new Set<ActionValue>();

  function fire(action: ActionValue) {
    if (isGameOver()) return;
    handleAction(action);
    render();
  }

  function process_repeats(now: number) {
    held.forEach((state, action) => {
      if (now - state.pressedAt < config.das_ms) return;
      const interval = config.arr_ms ?? 10;
      if (now - state.lastFire >= interval) {
        state.lastFire = now;
        fire(action);
      }
    });
  }

  let raf_id: number | null = null;
  function loop(now: number) {
    process_repeats(now);
    raf_id = window.requestAnimationFrame(loop);
  }

  function on_key_down(e: KeyboardEvent) {
    if (prevent_keys.has(e.key)) e.preventDefault();

    if (e.key === 'Escape') {
      targets.onPause?.();
      return;
    }

    if (isGameOver()) return;

    const action = key_map[e.key];
    if (action === undefined) return;

    if (REPEAT_ACTIONS.has(action)) {
      if (!held.has(action)) {
        held.set(action, { pressedAt: performance.now(), lastFire: performance.now() });
        targets.onPress?.(action);
        fire(action);
      }
    } else {
      if (!one_shot_active.has(action)) {
        one_shot_active.add(action);
        targets.onPress?.(action);
        fire(action);
      }
    }
  }

  function on_key_up(e: KeyboardEvent) {
    if (prevent_keys.has(e.key)) e.preventDefault();

    const action = key_map[e.key];
    if (action === undefined) return;

    if (REPEAT_ACTIONS.has(action)) {
      held.delete(action);
    } else {
      one_shot_active.delete(action);
    }
    targets.onRelease?.(action);
  }

  window.addEventListener('keydown', on_key_down);
  window.addEventListener('keyup', on_key_up);
  if (raf_id === null) raf_id = window.requestAnimationFrame(loop);

  return () => {
    window.removeEventListener('keydown', on_key_down);
    window.removeEventListener('keyup', on_key_up);
    if (raf_id !== null) {
      window.cancelAnimationFrame(raf_id);
      raf_id = null;
    }
  };
}

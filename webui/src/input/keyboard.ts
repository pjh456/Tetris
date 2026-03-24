import { Actions, type ActionValue } from '../game/actions';

type KeyboardTargets = {
  handleAction: (action: ActionValue) => void;
  isGameOver: () => boolean;
  render: () => void;
};

export function bindKeyboard(targets: KeyboardTargets) {
  const { handleAction, isGameOver, render } = targets;

  const repeatDelayMs = 150;
  const repeatIntervalMs: Record<ActionValue, number> = {
    [Actions.MoveLeft]: 60,
    [Actions.MoveRight]: 60,
    [Actions.SoftDrop]: 45,
    [Actions.HardDrop]: 0,
    [Actions.RotateCW]: 0,
    [Actions.RotateCCW]: 0,
    [Actions.Hold]: 0
  };

  type HoldState = { pressedAt: number; lastFire: number };
  const held = new Map<ActionValue, HoldState>();
  const oneShotActive = new Set<ActionValue>();

  function fire(action: ActionValue) {
    if (isGameOver()) return;
    handleAction(action);
    render();
  }

  function processRepeats(now: number) {
    held.forEach((state, action) => {
      const interval = repeatIntervalMs[action] ?? 40;
      if (now - state.pressedAt < repeatDelayMs) return;
      if (now - state.lastFire >= interval) {
        state.lastFire = now;
        fire(action);
      }
    });
  }

  let rafId: number | null = null;
  function loop(now: number) {
    processRepeats(now);
    rafId = window.requestAnimationFrame(loop);
  }

  function onKeyDown(e: KeyboardEvent) {
    if (isGameOver()) return;
    switch (e.key) {
      case 'ArrowLeft':
      case 'a':
        e.preventDefault();
        if (!held.has(Actions.MoveLeft)) {
          held.set(Actions.MoveLeft, { pressedAt: performance.now(), lastFire: performance.now() });
          fire(Actions.MoveLeft);
        }
        break;
      case 'ArrowRight':
      case 'd':
        e.preventDefault();
        if (!held.has(Actions.MoveRight)) {
          held.set(Actions.MoveRight, { pressedAt: performance.now(), lastFire: performance.now() });
          fire(Actions.MoveRight);
        }
        break;
      case 'ArrowDown':
      case 's':
        e.preventDefault();
        if (!held.has(Actions.SoftDrop)) {
          held.set(Actions.SoftDrop, { pressedAt: performance.now(), lastFire: performance.now() });
          fire(Actions.SoftDrop);
        }
        break;
      case 'ArrowUp':
      case 'w':
        e.preventDefault();
        if (!oneShotActive.has(Actions.RotateCW)) {
          oneShotActive.add(Actions.RotateCW);
          fire(Actions.RotateCW);
        }
        break;
      case ' ':
        e.preventDefault();
        if (!oneShotActive.has(Actions.HardDrop)) {
          oneShotActive.add(Actions.HardDrop);
          fire(Actions.HardDrop);
        }
        break;
      case 'c':
        e.preventDefault();
        if (!oneShotActive.has(Actions.Hold)) {
          oneShotActive.add(Actions.Hold);
          fire(Actions.Hold);
        }
        break;
      default:
        return;
    }
  }

  function onKeyUp(e: KeyboardEvent) {
    switch (e.key) {
      case 'ArrowLeft':
      case 'a':
        e.preventDefault();
        held.delete(Actions.MoveLeft);
        break;
      case 'ArrowRight':
      case 'd':
        e.preventDefault();
        held.delete(Actions.MoveRight);
        break;
      case 'ArrowDown':
      case 's':
        e.preventDefault();
        held.delete(Actions.SoftDrop);
        break;
      case 'ArrowUp':
      case 'w':
        e.preventDefault();
        oneShotActive.delete(Actions.RotateCW);
        break;
      case ' ':
        e.preventDefault();
        oneShotActive.delete(Actions.HardDrop);
        break;
      case 'c':
        e.preventDefault();
        oneShotActive.delete(Actions.Hold);
        break;
      default:
        break;
    }
  }

  window.addEventListener('keydown', onKeyDown);
  window.addEventListener('keyup', onKeyUp);
  if (rafId === null) rafId = window.requestAnimationFrame(loop);

  return () => {
    window.removeEventListener('keydown', onKeyDown);
    window.removeEventListener('keyup', onKeyUp);
    if (rafId !== null) {
      window.cancelAnimationFrame(rafId);
      rafId = null;
    }
  };
}

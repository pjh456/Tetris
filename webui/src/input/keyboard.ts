import { Actions, type ActionValue } from '../game/actions';

type KeyboardTargets = {
  handleAction: (action: ActionValue) => void;
  isGameOver: () => boolean;
  render: () => void;
};

export function bindKeyboard(targets: KeyboardTargets) {
  const { handleAction, isGameOver, render } = targets;

  function onKeyDown(e: KeyboardEvent) {
    if (isGameOver()) return;
    switch (e.key) {
      case 'ArrowLeft':
      case 'a':
        handleAction(Actions.MoveLeft);
        break;
      case 'ArrowRight':
      case 'd':
        handleAction(Actions.MoveRight);
        break;
      case 'ArrowDown':
      case 's':
        handleAction(Actions.SoftDrop);
        break;
      case 'ArrowUp':
      case 'w':
        handleAction(Actions.RotateCW);
        break;
      case ' ':
        handleAction(Actions.HardDrop);
        break;
      case 'c':
        handleAction(Actions.Hold);
        break;
      default:
        return;
    }
    render();
  }

  window.addEventListener('keydown', onKeyDown);

  return () => {
    window.removeEventListener('keydown', onKeyDown);
  };
}

export function shake_screen(element: HTMLElement): void {
  const offsets = [-8, 8, -4, 4, -2, 0];
  const step_ms = 33;

  offsets.forEach((offset, i) => {
    setTimeout(() => {
      element.style.transform = `translateX(${offset}px)`;
    }, step_ms * i);
  });
}

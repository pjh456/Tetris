let pending: ReturnType<typeof setTimeout>[] = [];

export function shake_screen(element: HTMLElement): void {
  for (const id of pending) clearTimeout(id);
  pending.length = 0;

  const offsets = [-8, 8, -4, 4, -2, 0];
  const step_ms = 33;

  offsets.forEach((offset, i) => {
    const id = setTimeout(() => {
      element.style.transform = `translateX(${offset}px)`;
    }, step_ms * i);
    pending.push(id);
  });
  setTimeout(() => { pending.length = 0; }, step_ms * offsets.length);
}

export function run_collapse_animation(
  canvas: HTMLCanvasElement,
  on_complete: () => void,
): () => void {
  const ctx = canvas.getContext('2d')!;
  ctx.save();
  ctx.resetTransform();

  const buf_w = canvas.width;
  const buf_h = canvas.height;
  const rows = 20;
  const cell_h = buf_h / rows;
  let current_row = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;

  function collapse_next() {
    if (current_row >= rows) {
      ctx.restore();
      on_complete();
      return;
    }
    ctx.fillStyle = 'rgba(50, 50, 50, 0.7)';
    ctx.fillRect(0, current_row * cell_h, buf_w, cell_h);
    current_row++;
    timer = setTimeout(collapse_next, 80);
  }
  collapse_next();

  return () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    try { ctx.restore(); } catch { /* already restored */ }
  };
}

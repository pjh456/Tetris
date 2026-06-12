export function run_collapse_animation(canvas: HTMLCanvasElement, on_complete: () => void): void {
  const ctx = canvas.getContext('2d')!;
  const css_h = parseFloat(canvas.style.height) || canvas.height;
  const rows = 20;
  const cell_h = css_h / rows;
  const css_w = parseFloat(canvas.style.width) || canvas.width;
  let current_row = 0;

  function collapse_next() {
    if (current_row >= rows) {
      on_complete();
      return;
    }
    ctx.fillStyle = 'rgba(50, 50, 50, 0.7)';
    ctx.fillRect(0, current_row * cell_h, css_w, cell_h);
    current_row++;
    setTimeout(collapse_next, 80);
  }
  collapse_next();
}

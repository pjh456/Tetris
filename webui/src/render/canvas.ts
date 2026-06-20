/** 读取 canvas 的 2D 上下文与 CSS 尺寸 / dpr（不修改画布尺寸）。 */
export function setup_canvas_ctx(canvas: HTMLCanvasElement): {
  ctx: CanvasRenderingContext2D;
  css_w: number;
  css_h: number;
  dpr: number;
} {
  const ctx = canvas.getContext('2d')!;
  const dpr = window.devicePixelRatio || 1;
  const css_w = parseFloat(canvas.style.width) || canvas.width / dpr;
  const css_h = parseFloat(canvas.style.height) || canvas.height / dpr;
  return { ctx, css_w, css_h, dpr };
}

/** 按 dpr 配置 HiDPI 画布（设位图尺寸 + CSS 尺寸 + scale），返回已缩放的 ctx。 */
export function setup_hidpi_canvas(
  canvas: HTMLCanvasElement,
  css_w: number,
  css_h: number,
): CanvasRenderingContext2D {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = css_w * dpr;
  canvas.height = css_h * dpr;
  canvas.style.width = `${css_w}px`;
  canvas.style.height = `${css_h}px`;
  const ctx = canvas.getContext('2d')!;
  ctx.scale(dpr, dpr);
  return ctx;
}

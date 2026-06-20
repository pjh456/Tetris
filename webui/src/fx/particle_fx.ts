export interface FxParticle {
  /** 推进一帧，返回是否仍存活。 */
  update(dt: number): boolean;
  draw(ctx: CanvasRenderingContext2D): void;
}

/**
 * 粒子特效基类：持有 canvas/ctx 与粒子数组，提供通用 update（推进 + 剔除死粒子）
 * 与 render（逐粒子绘制，存取 globalAlpha）。子类负责生成粒子与定义单粒子行为。
 * 不在 render 内 clearRect —— 需要清画布的子类（如 LineFx）自行 override。
 */
export class ParticleFx<P extends FxParticle> {
  protected canvas: HTMLCanvasElement;
  protected ctx: CanvasRenderingContext2D;
  protected particles: P[] = [];

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D context not available');
    this.ctx = ctx;
  }

  protected get css_w(): number {
    return parseFloat(this.canvas.style.width) || this.canvas.clientWidth || this.canvas.width;
  }

  protected add(p: P) {
    this.particles.push(p);
  }

  update(dt: number) {
    let write = 0;
    for (const p of this.particles) {
      if (p.update(dt)) this.particles[write++] = p;
    }
    this.particles.length = write;
  }

  render() {
    const prev_alpha = this.ctx.globalAlpha;
    for (const p of this.particles) {
      p.draw(this.ctx);
    }
    this.ctx.globalAlpha = prev_alpha;
  }

  clear() {
    this.particles.length = 0;
  }
}

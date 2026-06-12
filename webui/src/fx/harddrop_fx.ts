type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  life: number;
  max_life: number;
  color: string;
};

export class HardDropFx {
  private ctx: CanvasRenderingContext2D;
  private particles: Particle[] = [];

  constructor(private canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D context not available');
    this.ctx = ctx;
  }

  trigger(mask: number, y_start: number, y_end: number, color: string) {
    const css_w = parseFloat(this.canvas.style.width) || this.canvas.width;
    const cell = css_w / 10;
    const min_y = Math.min(y_start, y_end);
    const max_y = Math.max(y_start, y_end);

    for (let col = 0; col < 10; col++) {
      if (!(mask & (1 << col))) continue;
      const x_base = col * cell + cell * 0.5;
      for (let i = 0; i < 6; i++) {
        this.particles.push({
          x: x_base,
          y: min_y + Math.random() * (max_y - min_y + cell),
          vx: (Math.random() < 0.5 ? -1 : 1) * (0.08 + Math.random() * 0.15),
          vy: 0,
          size: 2 + Math.random() * 2,
          max_life: 250 + Math.random() * 100,
          life: 250 + Math.random() * 100,
          color,
        });
      }
    }
  }

  update(dt: number) {
    for (let i = this.particles.length - 1; i >= 0; i--) {
      const p = this.particles[i];
      p.life -= dt;
      if (p.life <= 0) {
        this.particles.splice(i, 1);
        continue;
      }
      p.x += p.vx * dt;
      p.y += p.vy * dt;
    }
  }

  render() {
    for (const p of this.particles) {
      const alpha = p.life / p.max_life;
      this.ctx.globalAlpha = alpha;
      this.ctx.fillStyle = p.color;
      this.ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
    }
    this.ctx.globalAlpha = 1;
  }
}

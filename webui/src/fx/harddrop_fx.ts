import { type FxParticle, ParticleFx } from './particle_fx';

class HardDropParticle implements FxParticle {
  x = 0;
  y = 0;
  vx = 0;
  vy = 0;
  size = 2;
  life = 0;
  max_life = 0;
  color = '';

  update(dt: number): boolean {
    this.life -= dt;
    if (this.life <= 0) return false;
    this.x += this.vx * dt;
    this.y += this.vy * dt;
    return true;
  }

  draw(ctx: CanvasRenderingContext2D): void {
    ctx.globalAlpha = this.life / this.max_life;
    ctx.fillStyle = this.color;
    ctx.fillRect(this.x - this.size / 2, this.y - this.size / 2, this.size, this.size);
  }
}

export class HardDropFx extends ParticleFx<HardDropParticle> {
  trigger(mask: number, y_start: number, y_end: number, color: string) {
    const cell = this.css_w / 10;
    const min_y = Math.min(y_start, y_end);
    const max_y = Math.max(y_start, y_end);

    for (let col = 0; col < 10; col++) {
      if (!(mask & (1 << col))) continue;
      const x_base = col * cell + cell * 0.5;
      for (let i = 0; i < 6; i++) {
        const p = new HardDropParticle();
        p.x = x_base;
        p.y = min_y + Math.random() * (max_y - min_y + cell);
        p.vx = (Math.random() < 0.5 ? -1 : 1) * (0.08 + Math.random() * 0.15);
        p.vy = 0;
        p.size = 2 + Math.random() * 2;
        p.max_life = 250 + Math.random() * 100;
        p.life = 250 + Math.random() * 100;
        p.color = color;
        this.add(p);
      }
    }
  }
}

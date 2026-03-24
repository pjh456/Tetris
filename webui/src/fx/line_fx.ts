type Flash = {
  y: number;
  height: number;
  ttl: number;
  life: number;
};

export class LineFx {
  private ctx: CanvasRenderingContext2D;
  private flashes: Flash[] = [];
  private particles: Particle[] = [];
  private pool: Particle[] = [];

  constructor(private canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D context not available');
    this.ctx = ctx;
  }

  resize(width: number, height: number) {
    this.canvas.width = width;
    this.canvas.height = height;
  }

  triggerFlash(y: number, height: number) {
    this.flashes.push({ y, height, ttl: 200, life: 200 });
    this.spawnParticles(y, height);
  }

  update(dt: number) {
    let write = 0;
    for (let i = 0; i < this.flashes.length; i++) {
      const f = this.flashes[i];
      f.ttl = Math.max(0, f.ttl - dt);
      if (f.ttl > 0) {
        this.flashes[write++] = f;
      }
    }
    this.flashes.length = write;

    const next: Particle[] = [];
    for (const p of this.particles) {
      p.life -= dt;
      if (p.life <= 0) {
        this.pool.push(p);
        continue;
      }
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.vy += 0.0009 * dt;
      next.push(p);
    }
    this.particles = next;
  }

  render() {
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    for (const f of this.flashes) {
      const t = f.ttl / f.life;
      const alpha = Math.max(0, t);
      ctx.fillStyle = `rgba(200, 240, 255, ${0.75 * alpha})`;
      ctx.fillRect(0, f.y, this.canvas.width, f.height);
      ctx.fillStyle = `rgba(255, 255, 255, ${0.45 * alpha})`;
      ctx.fillRect(0, f.y + f.height * 0.2, this.canvas.width, f.height * 0.6);
    }

    for (const p of this.particles) {
      const t = p.life / p.maxLife;
      const alpha = Math.max(0, t);
      ctx.fillStyle = p.color
        ? withAlpha(p.color, 0.9 * alpha)
        : `rgba(200, 240, 255, ${0.9 * alpha})`;
      ctx.fillRect(p.x, p.y, p.size, p.size * 0.6);
    }
  }

  private spawnParticles(y: number, height: number) {
    const count = 16;
    for (let i = 0; i < count; i++) {
      const p = this.pool.pop() ?? makeParticle();
      p.x = Math.random() * this.canvas.width;
      p.y = y + Math.random() * height;
      p.vx = (Math.random() - 0.5) * 0.3;
      p.vy = -0.35 - Math.random() * 0.25;
      p.size = 3 + Math.random() * 3;
      p.maxLife = 650 + Math.random() * 150;
      p.life = p.maxLife;
      p.color = '';
      this.particles.push(p);
    }
  }

  triggerColumnBurst(mask: number, yStart: number, yEnd: number, color: string) {
    const cols = 10;
    const cell = this.canvas.width / cols;
    const minY = Math.min(yStart, yEnd);
    const maxY = Math.max(yStart, yEnd);
    for (let col = 0; col < cols; col++) {
      if (!(mask & (1 << col))) continue;
      const xBase = col * cell + cell * 0.5;
      const count = 14;
      for (let i = 0; i < count; i++) {
        const p = this.pool.pop() ?? makeParticle();
        const side = Math.random() < 0.5 ? -1 : 1;
        p.x = xBase + side * (cell * 0.1 + Math.random() * cell * 0.2);
        p.y = minY + Math.random() * (maxY - minY + cell);
        p.vx = side * (0.1 + Math.random() * 0.12);
        p.vy = (Math.random() - 0.5) * 0.06;
        p.size = 2 + Math.random() * 1.5;
        p.maxLife = 480 + Math.random() * 180;
        p.life = p.maxLife;
        p.color = color;
        this.particles.push(p);
      }
    }
  }
}

type Particle = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  life: number;
  maxLife: number;
  color: string;
};

function makeParticle(): Particle {
  return {
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    size: 2,
    life: 0,
    maxLife: 0,
    color: ''
  };
}

function withAlpha(color: string, alpha: number) {
  if (color.startsWith('rgba(')) {
    return color.replace(/rgba\(([^)]+),\s*[\d.]+\)/, `rgba($1, ${alpha})`);
  }
  if (color.startsWith('rgb(')) {
    return color.replace(/rgb\(([^)]+)\)/, `rgba($1, ${alpha})`);
  }
  return color;
}

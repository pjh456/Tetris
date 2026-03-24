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
    this.flashes = this.flashes
      .map((f) => ({ ...f, ttl: Math.max(0, f.ttl - dt) }))
      .filter((f) => f.ttl > 0);

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
      ctx.fillStyle = `rgba(200, 240, 255, ${0.9 * alpha})`;
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
      this.particles.push(p);
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
};

function makeParticle(): Particle {
  return {
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    size: 2,
    life: 0,
    maxLife: 0
  };
}

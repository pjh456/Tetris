type Flash = {
  y: number;
  height: number;
  ttl: number;
  life: number;
};

export class LineFx {
  private ctx: CanvasRenderingContext2D;
  private flashes: Flash[] = [];

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
  }

  update(dt: number) {
    this.flashes = this.flashes
      .map((f) => ({ ...f, ttl: Math.max(0, f.ttl - dt) }))
      .filter((f) => f.ttl > 0);
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
  }
}

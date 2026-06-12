export type SfxEvent =
  | 'line_clear' | 'rotate' | 'hard_drop' | 't_spin'
  | 'game_over' | 'move' | 'soft_drop' | 'hold' | 'level_up';

const PRIORITY: Record<SfxEvent, number> = {
  t_spin: 3, game_over: 3,
  line_clear: 2,
  rotate: 1, hard_drop: 1, hold: 1, level_up: 1,
  move: 0, soft_drop: 0,
};

type SfxDef = { freq: number; freq_end: number; duration: number; type: OscillatorType };

const SFX_DEFS: Record<SfxEvent, SfxDef> = {
  move:       { freq: 200, freq_end: 200, duration: 0.03, type: 'square' },
  soft_drop:  { freq: 300, freq_end: 300, duration: 0.02, type: 'square' },
  rotate:     { freq: 400, freq_end: 600, duration: 0.05, type: 'triangle' },
  hold:       { freq: 500, freq_end: 700, duration: 0.08, type: 'triangle' },
  hard_drop:  { freq: 150, freq_end: 60,  duration: 0.1,  type: 'sine' },
  line_clear: { freq: 400, freq_end: 800, duration: 0.15, type: 'square' },
  level_up:   { freq: 400, freq_end: 800, duration: 0.2,  type: 'square' },
  t_spin:     { freq: 300, freq_end: 900, duration: 0.3,  type: 'sawtooth' },
  game_over:  { freq: 600, freq_end: 100, duration: 0.5,  type: 'sawtooth' },
};

export class AudioManager {
  private ctx: AudioContext | null = null;
  private sfx_gain: GainNode | null = null;
  private bgm_gain: GainNode | null = null;
  private bgm_source: AudioBufferSourceNode | null = null;
  private current_priority = -1;
  private current_timeout: ReturnType<typeof setTimeout> | null = null;
  private _initialized = false;
  private _bgm_volume = 0.5;

  async init(): Promise<void> {
    if (this._initialized) return;
    try {
      this.ctx = new AudioContext();
      this.sfx_gain = this.ctx.createGain();
      this.sfx_gain.connect(this.ctx.destination);
      this.bgm_gain = this.ctx.createGain();
      this.bgm_gain.connect(this.ctx.destination);
      this.sfx_gain.gain.value = 0.8;
      this.bgm_gain.gain.value = 0.5;
      this._initialized = true;
    } catch {
      // Audio not available
    }
  }

  play_sfx(event: SfxEvent): void {
    if (!this.ctx || !this._initialized) return;
    try {
      if (this.ctx.state === 'suspended') this.ctx.resume();

      const priority = PRIORITY[event];
      if (priority < this.current_priority) return;
      this.current_priority = priority;

      const def = SFX_DEFS[event];
      const osc = this.ctx.createOscillator();
      const gain = this.ctx.createGain();
      osc.type = def.type;
      osc.frequency.setValueAtTime(def.freq, this.ctx.currentTime);
      osc.frequency.linearRampToValueAtTime(def.freq_end, this.ctx.currentTime + def.duration);
      gain.gain.setValueAtTime(0.6, this.ctx.currentTime);
      gain.gain.linearRampToValueAtTime(0, this.ctx.currentTime + def.duration);
      osc.connect(gain);
      gain.connect(this.sfx_gain!);
      osc.start();
      osc.stop(this.ctx.currentTime + def.duration);

      if (this.bgm_gain && this.bgm_source) {
        this.bgm_gain.gain.linearRampToValueAtTime(
          this._bgm_volume * 0.5,
          this.ctx.currentTime + 0.05,
        );
      }

      if (this.current_timeout) clearTimeout(this.current_timeout);
      this.current_timeout = setTimeout(() => {
        this.current_priority = -1;
        if (this.bgm_gain && this.bgm_source && this.ctx) {
          this.bgm_gain.gain.linearRampToValueAtTime(
            this._bgm_volume,
            this.ctx.currentTime + 0.1,
          );
        }
      }, def.duration * 1000 + 50);
    } catch {
      // Fail silently
    }
  }

  start_bgm(): void {
    if (!this.ctx || !this._initialized) return;
    try {
      this.stop_bgm();
      const buf = this.ctx.createBuffer(1, this.ctx.sampleRate * 4, this.ctx.sampleRate);
      const data = buf.getChannelData(0);
      const notes = [131, 98, 110, 87];
      const note_samples = Math.floor(this.ctx.sampleRate);
      for (let n = 0; n < 4; n++) {
        const freq = notes[n];
        for (let i = 0; i < note_samples; i++) {
          const t = i / this.ctx.sampleRate;
          data[n * note_samples + i] = 0.3 * (((t * freq * 2) % 2) - 1);
        }
      }
      this.bgm_source = this.ctx.createBufferSource();
      this.bgm_source.buffer = buf;
      this.bgm_source.loop = true;
      this.bgm_source.connect(this.bgm_gain!);
      this.bgm_source.start();
    } catch {
      // Fail silently
    }
  }

  stop_bgm(): void {
    try { this.bgm_source?.stop(); } catch { /* */ }
    this.bgm_source = null;
  }

  set_sfx_volume(v: number): void {
    if (this.sfx_gain) this.sfx_gain.gain.value = Math.max(0, Math.min(1, v));
  }

  set_bgm_volume(v: number): void {
    this._bgm_volume = Math.max(0, Math.min(1, v));
    if (this.bgm_gain) this.bgm_gain.gain.value = this._bgm_volume;
  }

  dispose(): void {
    this.stop_bgm();
    try { this.ctx?.close(); } catch { /* */ }
    this.ctx = null;
    this._initialized = false;
  }
}

export const audio_manager = new AudioManager();

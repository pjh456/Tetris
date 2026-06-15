export type SfxEvent =
  | 'line_clear'
  | 'rotate'
  | 'hard_drop'
  | 't_spin'
  | 'game_over'
  | 'move'
  | 'soft_drop'
  | 'hold'
  | 'level_up';

const PRIORITY: Record<SfxEvent, number> = {
  t_spin: 3,
  game_over: 3,
  line_clear: 2,
  rotate: 1,
  hard_drop: 1,
  hold: 1,
  level_up: 1,
  move: 0,
  soft_drop: 0,
};

type SfxDef = { freq: number; freq_end: number; duration: number; type: OscillatorType };

const SFX_DEFS: Record<SfxEvent, SfxDef> = {
  move: { freq: 200, freq_end: 200, duration: 0.03, type: 'square' },
  soft_drop: { freq: 300, freq_end: 300, duration: 0.02, type: 'square' },
  rotate: { freq: 400, freq_end: 600, duration: 0.05, type: 'triangle' },
  hold: { freq: 500, freq_end: 700, duration: 0.08, type: 'triangle' },
  hard_drop: { freq: 150, freq_end: 60, duration: 0.1, type: 'sine' },
  line_clear: { freq: 500, freq_end: 1200, duration: 0.25, type: 'square' },
  level_up: { freq: 400, freq_end: 800, duration: 0.2, type: 'square' },
  t_spin: { freq: 300, freq_end: 900, duration: 0.3, type: 'sawtooth' },
  game_over: { freq: 600, freq_end: 100, duration: 0.5, type: 'sawtooth' },
};

type NoteEntry = [number, number]; // [frequency_hz, duration_in_eighths]

const KOROBEINIKI: NoteEntry[] = [
  [329.63, 2],
  [246.94, 1],
  [261.63, 1],
  [293.66, 2],
  [261.63, 1],
  [246.94, 1],
  [220.0, 2],
  [220.0, 1],
  [261.63, 1],
  [329.63, 2],
  [293.66, 1],
  [261.63, 1],
  [246.94, 2],
  [246.94, 1],
  [261.63, 1],
  [293.66, 2],
  [329.63, 2],
  [261.63, 2],
  [220.0, 2],
  [220.0, 2],
  [0, 2],
  [293.66, 2],
  [349.23, 1],
  [440.0, 2],
  [392.0, 1],
  [349.23, 1],
  [329.63, 2],
  [329.63, 1],
  [261.63, 1],
  [329.63, 2],
  [293.66, 1],
  [261.63, 1],
  [246.94, 2],
  [246.94, 1],
  [261.63, 1],
  [293.66, 2],
  [329.63, 2],
  [261.63, 2],
  [220.0, 2],
  [220.0, 2],
  [0, 2],
];

type ThemeAudio = 'cyberpunk' | 'retro' | 'minimal';

function wave_sample(phase: number, type: ThemeAudio): number {
  const p = phase % 1;
  switch (type) {
    case 'minimal':
      return p < 0.5 ? 0.4 : -0.4;
    case 'retro':
      return (1 - 4 * Math.abs(p - 0.5)) * 0.4;
    case 'cyberpunk':
      return (2 * p - 1) * 0.35;
  }
}

function render_melody(sample_rate: number, theme: ThemeAudio): AudioBuffer {
  const bpm = 140;
  const eighth = 60 / bpm / 2;
  let total_eighths = 0;
  for (const [, dur] of KOROBEINIKI) total_eighths += dur;
  const total_seconds = total_eighths * eighth;
  const total_samples = Math.ceil(total_seconds * sample_rate);

  const ctx = new OfflineAudioContext(1, total_samples, sample_rate);
  const buffer = ctx.createBuffer(1, total_samples, sample_rate);
  const data = buffer.getChannelData(0);

  let sample_offset = 0;
  for (const [freq, dur] of KOROBEINIKI) {
    const note_samples = Math.floor(dur * eighth * sample_rate);
    if (freq === 0) {
      sample_offset += note_samples;
      continue;
    }
    for (let i = 0; i < note_samples; i++) {
      const t = i / sample_rate;
      const env =
        i < note_samples * 0.05
          ? i / (note_samples * 0.05)
          : i > note_samples * 0.8
            ? (note_samples - i) / (note_samples * 0.2)
            : 1.0;
      const phase = t * freq;
      data[sample_offset + i] = wave_sample(phase, theme) * env;
    }
    sample_offset += note_samples;
  }

  return buffer;
}

export class AudioManager {
  private ctx: AudioContext | null = null;
  private sfx_gain: GainNode | null = null;
  private bgm_gain: GainNode | null = null;
  private bgm_filter: BiquadFilterNode | null = null;
  private bgm_delay: DelayNode | null = null;
  private bgm_feedback: GainNode | null = null;
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
      this.current_timeout = setTimeout(
        () => {
          this.current_priority = -1;
          if (this.bgm_gain && this.bgm_source && this.ctx) {
            this.bgm_gain.gain.linearRampToValueAtTime(
              this._bgm_volume,
              this.ctx.currentTime + 0.1,
            );
          }
        },
        def.duration * 1000 + 50,
      );
    } catch {
      // Fail silently
    }
  }

  start_bgm(theme: string = 'cyberpunk'): void {
    if (!this.ctx || !this._initialized) return;
    try {
      this.stop_bgm();
      const t =
        theme === 'retro' || theme === 'minimal' || theme === 'cyberpunk'
          ? (theme as ThemeAudio)
          : 'cyberpunk';

      const buffer = render_melody(this.ctx.sampleRate, t);
      this.bgm_source = this.ctx.createBufferSource();
      this.bgm_source.buffer = buffer;
      this.bgm_source.loop = true;

      let chain: AudioNode = this.bgm_source;

      if (t === 'cyberpunk') {
        this.bgm_filter = this.ctx.createBiquadFilter();
        this.bgm_filter.type = 'lowpass';
        this.bgm_filter.frequency.value = 2000;
        this.bgm_filter.Q.value = 5;
        chain.connect(this.bgm_filter);
        chain = this.bgm_filter;

        this.bgm_delay = this.ctx.createDelay(1);
        this.bgm_delay.delayTime.value = 0.25;
        const feedback = this.ctx.createGain();
        feedback.gain.value = 0.25;
        this.bgm_feedback = feedback;
        chain.connect(this.bgm_delay);
        this.bgm_delay.connect(feedback);
        feedback.connect(this.bgm_delay);
        this.bgm_delay.connect(this.bgm_gain!);
        chain.connect(this.bgm_gain!);
      } else if (t === 'retro') {
        this.bgm_filter = this.ctx.createBiquadFilter();
        this.bgm_filter.type = 'bandpass';
        this.bgm_filter.frequency.value = 800;
        this.bgm_filter.Q.value = 2;
        chain.connect(this.bgm_filter);
        chain = this.bgm_filter;
        chain.connect(this.bgm_gain!);
      } else {
        chain.connect(this.bgm_gain!);
      }

      this.bgm_source.start();
    } catch {
      // Fail silently
    }
  }

  stop_bgm(): void {
    try {
      this.bgm_source?.stop();
    } catch {
      /* */
    }
    try {
      this.bgm_filter?.disconnect();
    } catch {
      /* */
    }
    try {
      this.bgm_delay?.disconnect();
    } catch {
      /* */
    }
    try {
      this.bgm_feedback?.disconnect();
    } catch {
      /* */
    }
    this.bgm_source = null;
    this.bgm_filter = null;
    this.bgm_delay = null;
    this.bgm_feedback = null;
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
    try {
      this.ctx?.close();
    } catch {
      /* */
    }
    this.ctx = null;
    this._initialized = false;
  }
}

export const audio_manager = new AudioManager();

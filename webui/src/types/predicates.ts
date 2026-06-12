import type { HudData } from '../render/hud';

export type HardDropInfo = {
  cols: number;
  start_y: number;
  end_y: number;
  piece: number;
};

export function is_hud_data(data: unknown): data is HudData {
  if (typeof data !== 'object' || data === null) return false;
  const d = data as Record<string, unknown>;
  return (
    typeof d.score === 'number' &&
    typeof d.level === 'number' &&
    typeof d.lines === 'number' &&
    typeof d.combo === 'number' &&
    typeof d.b2b === 'number' &&
    typeof d.tspin === 'number' &&
    typeof d.all_clear === 'number'
  );
}

export function is_hard_drop_info(data: unknown): data is HardDropInfo {
  if (typeof data !== 'object' || data === null) return false;
  const d = data as Record<string, unknown>;
  return (
    typeof d.cols === 'number' &&
    d.cols > 0 &&
    typeof d.start_y === 'number' &&
    typeof d.end_y === 'number' &&
    typeof d.piece === 'number'
  );
}

export function is_valid_theme(s: unknown): s is 'cyberpunk' | 'retro' | 'minimal' {
  return s === 'cyberpunk' || s === 'retro' || s === 'minimal';
}

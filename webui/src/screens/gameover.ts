import { get_wasm } from '../core/wasm';
import { page, is_multiplayer, connection_status } from '../state';
import { audio_manager } from '../core/audio';
import { save_score_to_leaderboard } from '../core/settings_store';

function format_time(ms: number): string {
  const total_sec = Math.floor(ms / 1000);
  const min = Math.floor(total_sec / 60);
  const sec = total_sec % 60;
  return `${min}:${sec.toString().padStart(2, '0')}`;
}

export function create_gameover_screen(): HTMLElement {
  const el = document.createElement('div');
  el.className = 'gameover-page';

  audio_manager.play_sfx('game_over');
  audio_manager.stop_bgm();

  let stats_html: string;
  try {
    const wasm = get_wasm();
    const s = wasm.get_game_stats() as {
      score: number;
      lines: number;
      level: number;
      game_time_ms: number;
      max_combo: number;
      tspin_count: number;
      total_pieces: number;
    };
    const time_sec = s.game_time_ms / 1000;
    const pps = time_sec > 0 ? (s.total_pieces / time_sec).toFixed(1) : '0.0';
    const apm = time_sec > 0 ? Math.round((s.total_pieces / time_sec) * 60) : 0;
    const rank = save_score_to_leaderboard(s.score, s.level, s.lines);
    const rank_html =
      rank > 0
        ? `<div class="stat-row"><span class="stat-label">Rank</span><span class="stat-value stat-score">NEW #${rank}</span></div>`
        : '';

    stats_html = `${rank_html}
      <div class="stat-row"><span class="stat-label">Score</span><span class="stat-value stat-score">${s.score.toLocaleString()}</span></div>
      <div class="stat-row"><span class="stat-label">Level</span><span class="stat-value">${s.level}</span></div>
      <div class="stat-row"><span class="stat-label">Lines</span><span class="stat-value">${s.lines}</span></div>
      <div class="stat-row"><span class="stat-label">Time</span><span class="stat-value">${format_time(s.game_time_ms)}</span></div>
      <div class="stat-row"><span class="stat-label">Max Combo</span><span class="stat-value">${s.max_combo}</span></div>
      <div class="stat-row"><span class="stat-label">T-Spins</span><span class="stat-value">${s.tspin_count}</span></div>
      <div class="stat-row"><span class="stat-label">PPS</span><span class="stat-value">${pps}</span></div>
      <div class="stat-row"><span class="stat-label">APM</span><span class="stat-value">${apm}</span></div>
      <div class="stat-row"><span class="stat-label">Pieces</span><span class="stat-value">${s.total_pieces}</span></div>
    `;
  } catch {
    stats_html = '<div class="stat-row"><span class="stat-label">No stats available</span></div>';
  }

  el.innerHTML = `
    <div class="gameover-panel glass">
      <h1 class="gameover-title">GAME OVER</h1>
      <div class="gameover-stats">${stats_html}</div>
      <div class="gameover-buttons">
        <button class="btn" id="go-retry">再来一局</button>
        <button class="btn" id="go-new">新游戏</button>
        <button class="btn" id="go-menu">主菜单</button>
      </div>
    </div>
  `;

  el.querySelector('#go-retry')!.addEventListener('click', () => {
    // Retry: replay same mode (single/multi) with same settings
    page.value = 'game';
  });
  el.querySelector('#go-new')!.addEventListener('click', () => {
    is_multiplayer.value = false;
    connection_status.value = 'offline';
    page.value = 'game';
  });
  el.querySelector('#go-menu')!.addEventListener('click', () => {
    page.value = 'home';
  });

  return el;
}

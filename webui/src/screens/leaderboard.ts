import { page } from '../state';
import { get_leaderboard, clear_leaderboard, type LeaderboardEntry } from '../core/settings_store';

const RANK_COLORS = ['#ffd700', '#c0c0c0', '#cd7f32'];

export function create_leaderboard_screen(): HTMLElement {
  const overlay = document.createElement('div');
  overlay.className = 'leaderboard-overlay';
  overlay.setAttribute('role', 'dialog');
  overlay.setAttribute('aria-label', 'Top 10 Leaderboard');
  overlay.setAttribute('aria-modal', 'true');

  const panel = document.createElement('div');
  panel.className = 'leaderboard-panel glass';

  const entries = get_leaderboard();
  panel.appendChild(build_header());
  panel.appendChild(build_table(entries));
  panel.appendChild(build_best_rank(entries));
  panel.appendChild(build_buttons(overlay));

  overlay.appendChild(panel);

  overlay.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      page.value = 'home';
    }
  });

  setTimeout(() => panel.focus(), 0);
  panel.setAttribute('tabindex', '-1');

  return overlay;
}

function build_header(): HTMLElement {
  const header = document.createElement('div');
  header.className = 'leaderboard-header';
  header.innerHTML = `<h2>TOP 10 LEADERBOARD</h2><div class="leaderboard-divider"></div>`;
  return header;
}

function build_table(entries: LeaderboardEntry[]): HTMLElement {
  const table = document.createElement('div');
  table.className = 'leaderboard-table';

  if (entries.length === 0) {
    const empty = document.createElement('div');
    empty.className = 'leaderboard-empty';
    empty.textContent =
      'No Scores Yet — Complete a solo game to earn your spot on the leaderboard.';
    table.appendChild(empty);
    return table;
  }

  for (let i = 0; i < 10; i++) {
    const row = document.createElement('div');
    row.className = 'leaderboard-row';

    const rank = document.createElement('span');
    rank.className = 'leaderboard-rank';
    rank.textContent = `#${i + 1}`;
    if (i < 3) {
      rank.style.color = RANK_COLORS[i];
    }

    if (i < entries.length) {
      const e = entries[i];
      const score_el = document.createElement('span');
      score_el.className = 'leaderboard-score';
      score_el.textContent = e.score.toLocaleString('en-US');

      const level_el = document.createElement('span');
      level_el.className = 'leaderboard-level';
      level_el.textContent = `Lv.${e.level}`;

      const lines_el = document.createElement('span');
      lines_el.className = 'leaderboard-lines';
      lines_el.textContent = `${e.lines}L`;

      row.append(rank, score_el, level_el, lines_el);
    } else {
      const dash = document.createElement('span');
      dash.className = 'leaderboard-score';
      dash.textContent = '---';
      row.append(rank, dash);
    }

    table.appendChild(row);
  }

  return table;
}

function build_best_rank(entries: LeaderboardEntry[]): HTMLElement {
  const div = document.createElement('div');
  div.className = 'leaderboard-best';
  if (entries.length > 0) {
    const best = entries[0];
    div.textContent = `Your best: #1 · ${best.score.toLocaleString('en-US')}`;
  } else {
    div.textContent = 'Not ranked yet';
  }
  return div;
}

function build_buttons(overlay: HTMLElement): HTMLElement {
  const bar = document.createElement('div');
  bar.className = 'leaderboard-buttons';

  const play_btn = document.createElement('button');
  play_btn.className = 'btn';
  play_btn.textContent = 'Play Again';
  play_btn.addEventListener('click', () => {
    page.value = 'game';
  });

  const clear_btn = document.createElement('button');
  clear_btn.className = 'btn btn-danger';
  clear_btn.textContent = 'Clear All';
  clear_btn.addEventListener('click', () => {
    show_confirm(overlay);
  });

  const close_btn = document.createElement('button');
  close_btn.className = 'btn';
  close_btn.textContent = 'Close';
  close_btn.addEventListener('click', () => {
    page.value = 'home';
  });

  bar.append(play_btn, clear_btn, close_btn);
  return bar;
}

function show_confirm(overlay: HTMLElement) {
  const modal = document.createElement('div');
  modal.className = 'confirm-overlay';
  modal.innerHTML = `
    <div class="confirm-panel glass">
      <p>Clear All Scores? This cannot be undone.</p>
      <div class="confirm-buttons">
        <button class="btn" id="confirm-cancel">Cancel</button>
        <button class="btn btn-danger" id="confirm-clear">Clear</button>
      </div>
    </div>
  `;
  overlay.appendChild(modal);

  modal.querySelector('#confirm-cancel')!.addEventListener('click', () => {
    modal.remove();
  });
  modal.querySelector('#confirm-clear')!.addEventListener('click', () => {
    clear_leaderboard();
    modal.remove();
    page.value = 'leaderboard';
  });
}

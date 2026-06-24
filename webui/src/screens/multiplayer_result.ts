import { create_button } from '../core/dom';
import { page, match_standings, type StandingRow } from '../state';
import { reset_multiplayer_ws } from '../core/multiplayer';

export function create_multiplayer_result_screen(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'multiplayer-result';

  const title = document.createElement('h1');
  title.className = 'result-title';
  title.textContent = 'MATCH RESULTS';
  container.appendChild(title);

  const rows: StandingRow[] = [...match_standings.value].sort((a, b) => a.placement - b.placement);

  const winner = rows.find((row) => row.placement === 1);
  const winner_banner = document.createElement('div');
  winner_banner.className = 'result-winner';
  winner_banner.textContent = winner ? `WINNER: ${winner.name}` : 'DRAW';
  container.appendChild(winner_banner);

  const table = document.createElement('div');
  table.className = 'result-table';

  const header = document.createElement('div');
  header.className = 'result-row result-header';
  for (const col of ['#', 'PLAYER', 'SCORE', 'LINES', 'TIME']) {
    const cell = document.createElement('span');
    cell.className = 'result-cell';
    cell.textContent = col;
    header.appendChild(cell);
  }
  table.appendChild(header);

  for (const row of rows) {
    const tr = document.createElement('div');
    tr.className = 'result-row' + (row.placement === 1 ? ' result-row-winner' : '');
    const cells = [
      String(row.placement),
      row.name,
      String(row.score),
      String(row.lines),
      row.survival_ticks > 0 ? `${Math.round(row.survival_ticks / 60)}s` : '--',
    ];
    for (const value of cells) {
      const cell = document.createElement('span');
      cell.className = 'result-cell';
      cell.textContent = value;
      tr.appendChild(cell);
    }
    table.appendChild(tr);
  }
  container.appendChild(table);

  const back_btn = create_button('返回 Lobby', {
    ariaLabel: '返回 Lobby',
    onClick: () => {
      // Close the handed-off game ws so lobby reconnects cleanly (no ghost peer).
      reset_multiplayer_ws();
      page.value = 'lobby';
    },
  });
  back_btn.classList.add('result-back-btn');
  container.appendChild(back_btn);

  return container;
}

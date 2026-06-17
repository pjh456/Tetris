import { page } from '../state';

export interface SurvivingPlayer {
  id: number;
  name: string;
}

export function create_spectator_screen(players: SurvivingPlayer[]): HTMLElement {
  const container = document.createElement('div');
  container.className = 'spectator-view';

  let active_idx = 0;

  const tabs = build_tabs(players, active_idx, (idx) => {
    active_idx = idx;
    update_active_tab(tabs, idx);
  });
  container.appendChild(tabs);

  const board_container = document.createElement('div');
  board_container.className = 'spectator-board';
  board_container.textContent = 'Spectating: ' + (players[0]?.name ?? 'N/A');
  container.appendChild(board_container);

  const status = build_status(players[active_idx]);
  container.appendChild(status);

  container.addEventListener('keydown', (e) => {
    if (players.length === 0) return;
    if (e.key === 'Tab' || e.key === 'ArrowRight') {
      e.preventDefault();
      const next = e.shiftKey
        ? (active_idx - 1 + players.length) % players.length
        : (active_idx + 1) % players.length;
      active_idx = next;
      update_active_tab(tabs, active_idx);
      update_status(status, players[active_idx]);
    }
    if (e.key === 'Escape') {
      page.value = 'home';
    }
  });

  container.setAttribute('tabindex', '0');
  setTimeout(() => container.focus(), 0);

  return container;
}

function build_tabs(
  players: SurvivingPlayer[],
  active: number,
  on_select: (idx: number) => void,
): HTMLElement {
  const bar = document.createElement('div');
  bar.className = 'spectator-tabs';

  if (players.length <= 1) {
    bar.style.display = 'none';
    return bar;
  }

  for (let i = 0; i < players.length; i++) {
    const tab = document.createElement('div');
    tab.className = 'spectator-tab' + (i === active ? ' active' : '');
    tab.textContent = players[i].name;
    tab.addEventListener('click', () => on_select(i));
    bar.appendChild(tab);
  }

  return bar;
}

function update_active_tab(bar: HTMLElement, active: number) {
  const tabs = bar.querySelectorAll('.spectator-tab');
  tabs.forEach((t, i) => {
    t.classList.toggle('active', i === active);
  });
}

function build_status(player?: SurvivingPlayer): HTMLElement {
  const el = document.createElement('div');
  el.className = 'spectator-status';
  el.textContent = player ? `Spectating: ${player.name}` : 'No players remaining';
  return el;
}

function update_status(el: HTMLElement, player?: SurvivingPlayer) {
  el.textContent = player ? `Spectating: ${player.name}` : 'No players remaining';
}

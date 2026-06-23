import { page, match_standings, type StandingRow } from '../state';
import {
  consume_last_multiplayer_event,
  get_multiplayer_snapshot,
  get_opponent_player_grid,
  type MultiplayerEvent,
} from '../core/wasm';
import { get_multiplayer_ws } from '../core/multiplayer';
import { create_readonly_board_view } from '../render/readonly_board';

interface SurvivingPlayer {
  player_id: number;
  name: string;
}

function surviving_players(): SurvivingPlayer[] {
  const snapshot = get_multiplayer_snapshot();
  return (snapshot?.opponents ?? [])
    .filter((player) => player.alive && !player.away)
    .map((player) => ({ player_id: player.player_id, name: player.name }));
}

export function create_spectator_screen(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'spectator-view';
  container.setAttribute('tabindex', '0');

  let players = surviving_players();
  let active_idx = 0;

  const tabs = document.createElement('div');
  tabs.className = 'spectator-tabs';
  container.appendChild(tabs);

  const board_container = document.createElement('div');
  board_container.className = 'spectator-board';
  container.appendChild(board_container);

  const status = document.createElement('div');
  status.className = 'spectator-status';
  container.appendChild(status);

  const board = create_readonly_board_view({
    width: 360,
    height: 720,
    get_grid: () => {
      const player = players[active_idx];
      return player ? get_opponent_player_grid(player.player_id) : null;
    },
  });
  board.mount(board_container);

  function render_tabs() {
    tabs.innerHTML = '';
    tabs.style.display = players.length > 1 ? '' : 'none';
    players.forEach((player, i) => {
      const tab = document.createElement('div');
      tab.className = 'spectator-tab' + (i === active_idx ? ' active' : '');
      tab.textContent = player.name;
      tab.addEventListener('click', () => {
        active_idx = i;
        render_tabs();
        render_status();
      });
      tabs.appendChild(tab);
    });
  }

  function render_status() {
    const player = players[active_idx];
    status.textContent = player ? `Spectating: ${player.name}` : 'No players remaining';
  }

  function refresh_players() {
    players = surviving_players();
    if (active_idx >= players.length) active_idx = Math.max(0, players.length - 1);
    render_tabs();
    render_status();
  }

  render_tabs();
  render_status();

  const ws = get_multiplayer_ws();
  if (ws) {
    // ws_client already advanced opponent engines via parse_packet before this
    // handler runs, so the read-only board reflects live opponent state.
    ws.onpacket = (event: MultiplayerEvent | null) => {
      const last_event = event ?? consume_last_multiplayer_event();
      if (last_event?.kind === 'standings') {
        match_standings.value = (last_event.standings ?? []) as StandingRow[];
        page.value = 'multiplayer_result';
        return;
      }
      refresh_players();
    };
  }

  let raf_id = 0;
  function loop() {
    board.render();
    raf_id = requestAnimationFrame(loop);
  }
  raf_id = requestAnimationFrame(loop);

  container.addEventListener('keydown', (e) => {
    if (
      players.length > 1 &&
      (e.key === 'Tab' || e.key === 'ArrowRight' || e.key === 'ArrowLeft')
    ) {
      e.preventDefault();
      const back = e.shiftKey || e.key === 'ArrowLeft';
      active_idx = back
        ? (active_idx - 1 + players.length) % players.length
        : (active_idx + 1) % players.length;
      render_tabs();
      render_status();
    }
    if (e.key === 'Escape') {
      page.value = 'home';
    }
  });

  const focus_timer = setTimeout(() => container.focus(), 0);

  const el = container as HTMLElement & { _cleanup?: () => void };
  el._cleanup = () => {
    clearTimeout(focus_timer);
    if (raf_id) cancelAnimationFrame(raf_id);
    if (ws && ws.onpacket) ws.onpacket = null;
    board.destroy();
  };
  return el;
}

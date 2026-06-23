import { effect } from '@preact/signals-core';
import { create_button } from '../core/dom';
import { page, is_multiplayer, room_code, connection_status } from '../state';
import { WsClient } from '../core/ws_client';
import { set_multiplayer_ws } from '../core/multiplayer';
import { add_multiplayer_ai_opponent } from '../core/ai_opponent';
import { get_display_name } from '../core/user_profile';
import {
  consume_last_multiplayer_event,
  get_multiplayer_snapshot,
  get_wasm,
  make_chat_message_packet,
  make_join_room_packet,
  make_kick_player_packet,
  make_player_ready_packet,
  make_remove_bot_packet,
  reset_multiplayer_wasm,
} from '../core/wasm';
import type { MultiplayerPlayer } from '../core/wasm';

const RELAY_URL = 'ws://localhost:9000';
const CODE_CHARS = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';

function gen_room_code(): string {
  return Array.from(
    { length: 4 },
    () => CODE_CHARS[Math.floor(Math.random() * CODE_CHARS.length)],
  ).join('');
}

export function create_lobby_screen(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'lobby';

  if (!room_code.value) {
    room_code.value = gen_room_code();
  }

  const wasm = get_wasm();
  let current_ws = new WsClient(`${RELAY_URL}/room/${room_code.value}`, wasm);
  let peers: MultiplayerPlayer[] = [];
  const my_name = get_display_name();
  let is_ready = false;
  let last_room_sync_count: number | null = null;
  let am_host = false;
  let my_player_id: number | null = null;

  const layout = document.createElement('div');
  layout.className = 'lobby-layout';

  const main = document.createElement('div');
  main.className = 'lobby-main';

  const { section: room_section, join_input, join_btn } = build_room_code_section();
  const { section: player_section, update: update_players } = build_player_list({
    is_host: () => am_host,
    local_id: () => my_player_id,
    on_kick: (player_id) => current_ws.send(make_kick_player_packet(player_id)),
    on_remove_bot: (player_id) => current_ws.send(make_remove_bot_packet(player_id)),
  });
  const { btn: ready_btn, set_ready: set_ready_btn } = build_ready_button();
  const add_ai_btn = build_add_ai_button();

  main.appendChild(room_section);
  main.appendChild(player_section);
  main.appendChild(add_ai_btn);
  main.appendChild(ready_btn);

  const sidebar = document.createElement('div');
  sidebar.className = 'lobby-sidebar';
  const {
    section: chat_section,
    append_system,
    append_chat,
  } = build_chat_section((text) => {
    current_ws.send(make_chat_message_packet(text));
  });
  sidebar.appendChild(chat_section);

  layout.appendChild(main);
  layout.appendChild(sidebar);
  container.appendChild(layout);

  const countdown_el = document.createElement('div');
  countdown_el.className = 'lobby-label';
  countdown_el.textContent = '';
  main.appendChild(countdown_el);

  function attach_handlers(ws: WsClient) {
    ws.onopen = () => {
      ws.send(make_join_room_packet(room_code.value ?? '----', my_name));
    };
    ws.onpacket = (event) => {
      const last_event = event ?? consume_last_multiplayer_event();
      const snapshot = get_multiplayer_snapshot();
      if (snapshot) {
        peers = snapshot.players;
        const me =
          typeof snapshot.local_player_id === 'number'
            ? peers.find((player) => player.player_id === snapshot.local_player_id)
            : peers.find((player) => player.name === my_name);
        my_player_id = me?.player_id ?? null;
        am_host = me?.is_host ?? false;
        update_players(peers);
        room_code.value = snapshot.room_code ?? room_code.value;
        const code_display = room_section.querySelector('.room-code');
        if (code_display) code_display.textContent = room_code.value ?? '----';
        is_ready = me?.ready ?? false;
        set_ready_btn(is_ready);
        countdown_el.textContent =
          snapshot.countdown !== null && snapshot.countdown !== undefined
            ? `STARTING IN ${snapshot.countdown}`
            : '';
      }
      if (!last_event) {
        return;
      }
      if (last_event.kind === 'chat' && last_event.message) {
        const speaker =
          peers.find((player) => player.player_id === last_event.player_id)?.name ?? 'Player';
        append_chat(speaker, last_event.message);
      } else if (last_event.kind === 'room_snapshot') {
        if (last_room_sync_count !== peers.length) {
          append_system(`Room synced: ${peers.length} player(s)`);
          last_room_sync_count = peers.length;
        }
      } else if (last_event.kind === 'countdown' && typeof last_event.countdown === 'number') {
        countdown_el.textContent = `STARTING IN ${last_event.countdown}`;
      } else if (last_event.kind === 'countdown_cancel') {
        countdown_el.textContent = '';
      } else if (last_event.kind === 'kicked' && last_event.player_id === my_player_id) {
        cleanup();
        page.value = 'home';
      } else if (last_event.kind === 'game_start' && typeof last_event.random_seed === 'number') {
        is_multiplayer.value = true;
        set_multiplayer_ws(current_ws);
        reset_multiplayer_wasm(last_event.random_seed);
        dispose();
        page.value = 'game';
      }
    };
  }

  function connect_to(code: string) {
    current_ws.close();
    room_code.value = code;
    const code_display = room_section.querySelector('.room-code');
    if (code_display) code_display.textContent = code;
    peers.length = 0;
    is_ready = false;
    last_room_sync_count = null;
    set_ready_btn(false);
    update_players([]);
    current_ws = new WsClient(`${RELAY_URL}/room/${code}`, wasm);
    attach_handlers(current_ws);
    current_ws.connect();
  }

  ready_btn.addEventListener('click', () => {
    is_ready = !is_ready;
    set_ready_btn(is_ready);
    if (!is_ready) {
      countdown_el.textContent = '';
    }
    current_ws.send(make_player_ready_packet(is_ready));
  });

  add_ai_btn.addEventListener('click', () => {
    if (add_multiplayer_ai_opponent(current_ws)) {
      append_system('AI opponent requested');
    } else {
      append_system('Relay not connected; AI request not sent');
    }
  });

  attach_handlers(current_ws);
  current_ws.connect();

  join_btn.addEventListener('click', () => {
    const code = join_input.value.trim().toUpperCase();
    if (code.length === 4) {
      join_input.value = '';
      connect_to(code);
    }
  });

  join_input.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      const code = join_input.value.trim().toUpperCase();
      if (code.length === 4) {
        join_input.value = '';
        connect_to(code);
      }
    }
  });

  const dispose = effect(() => {
    const dot = room_section.querySelector('.status-dot');
    if (dot) {
      dot.className = `status-dot status-${connection_status.value}`;
    }
  });

  function cleanup() {
    dispose();
    // Only close ws if we didn't hand it off to the game
    if (!is_multiplayer.value) {
      current_ws.close();
      set_multiplayer_ws(null);
    }
  }

  container.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      cleanup();
      page.value = 'home';
    }
  });

  (container as HTMLElement & { _cleanup?: () => void })._cleanup = cleanup;

  return container;
}

function build_add_ai_button(): HTMLButtonElement {
  return create_button('加 AI', { ariaLabel: '加 AI 对手' });
}

function build_room_code_section(): {
  section: HTMLElement;
  join_input: HTMLInputElement;
  join_btn: HTMLButtonElement;
} {
  const section = document.createElement('div');
  section.className = 'lobby-section';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'ROOM CODE';

  const code_display = document.createElement('div');
  code_display.className = 'room-code';
  code_display.textContent = room_code.value ?? '----';

  const copy_btn = create_button('Copy', {
    onClick: () => {
      if (room_code.value) {
        navigator.clipboard.writeText(room_code.value).catch(() => {});
      }
    },
  });

  const join_row = document.createElement('div');
  join_row.className = 'join-row';

  const join_input = document.createElement('input');
  join_input.type = 'text';
  join_input.className = 'join-input';
  join_input.maxLength = 4;
  join_input.placeholder = 'Room code';
  join_input.style.textTransform = 'uppercase';

  const join_btn = document.createElement('button');
  join_btn.className = 'btn join-btn';
  join_btn.textContent = 'Join';

  join_row.append(join_input, join_btn);

  const status = document.createElement('span');
  status.className = 'status-dot status-offline';

  section.append(label, code_display, status, copy_btn, join_row);
  return { section, join_input, join_btn };
}

function build_player_list(opts: {
  is_host: () => boolean;
  local_id: () => number | null;
  on_kick: (player_id: number) => void;
  on_remove_bot: (player_id: number) => void;
}): {
  section: HTMLElement;
  update: (peers: MultiplayerPlayer[]) => void;
} {
  const section = document.createElement('div');
  section.className = 'lobby-section player-list';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'PLAYERS';
  section.appendChild(label);

  const cards_container = document.createElement('div');
  section.appendChild(cards_container);

  function render(peers: MultiplayerPlayer[]) {
    cards_container.innerHTML = '';
    const slots = Math.max(peers.length, 1);
    for (let i = 0; i < slots; i++) {
      const card = document.createElement('div');
      card.className = 'player-card glass';
      if (peers[i]) {
        const player = peers[i];
        const status_class = player.ready ? 'player-status player-ready' : 'player-status';
        const status_text = player.ready ? '✓ READY' : player.away ? 'AWAY' : 'Not Ready';
        const host_tag = player.is_host ? ' (HOST)' : '';
        const name_span = document.createElement('span');
        name_span.className = 'player-name';
        name_span.textContent = player.name + host_tag;
        card.appendChild(name_span);
        const status_span = document.createElement('span');
        status_span.className = status_class;
        status_span.textContent = status_text;
        card.appendChild(status_span);
        const is_self = player.player_id === opts.local_id();
        if (opts.is_host() && !is_self) {
          if (player.is_bot) {
            const remove_btn = create_button('删 AI', {
              ariaLabel: `删除 ${player.name}`,
              onClick: () => opts.on_remove_bot(player.player_id),
            });
            remove_btn.classList.add('player-card-action');
            card.appendChild(remove_btn);
          } else {
            const kick_btn = create_button('踢出', {
              ariaLabel: `踢出 ${player.name}`,
              onClick: () => opts.on_kick(player.player_id),
            });
            kick_btn.classList.add('player-card-action');
            card.appendChild(kick_btn);
          }
        }
      } else {
        const name_span = document.createElement('span');
        name_span.className = 'player-name';
        name_span.textContent = '\u2014';
        card.appendChild(name_span);
        const status_span = document.createElement('span');
        status_span.className = 'player-status';
        status_span.textContent = 'Waiting...';
        card.appendChild(status_span);
      }
      cards_container.appendChild(card);
    }
  }

  render([]);

  return { section, update: render };
}

function build_ready_button(): {
  btn: HTMLButtonElement;
  set_ready: (r: boolean) => void;
} {
  const btn = document.createElement('button');
  btn.className = 'btn ready-btn';
  btn.textContent = 'READY';

  function set_ready(r: boolean) {
    if (r) {
      btn.textContent = 'CANCEL READY';
      btn.classList.add('ready-btn-active');
    } else {
      btn.textContent = 'READY';
      btn.classList.remove('ready-btn-active');
    }
  }

  return { btn, set_ready };
}

function build_chat_section(on_send: (text: string) => void): {
  section: HTMLElement;
  append_system: (text: string) => void;
  append_chat: (from: string, text: string) => void;
} {
  const section = document.createElement('div');
  section.className = 'lobby-section chat-area';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'CHAT';
  section.appendChild(label);

  const messages = document.createElement('div');
  messages.className = 'chat-messages';
  messages.innerHTML = '<div class="chat-empty">No messages yet. Say hello!</div>';
  section.appendChild(messages);

  function scroll_bottom() {
    messages.scrollTop = messages.scrollHeight;
  }

  function append_system(text: string) {
    const empty = messages.querySelector('.chat-empty');
    if (empty) empty.remove();
    const el = document.createElement('div');
    el.className = 'chat-message chat-system';
    el.textContent = text;
    messages.appendChild(el);
    scroll_bottom();
  }

  function append_chat(from: string, text: string) {
    const empty = messages.querySelector('.chat-empty');
    if (empty) empty.remove();
    const el = document.createElement('div');
    el.className = 'chat-message';
    el.textContent = `${from}: ${text}`;
    messages.appendChild(el);
    scroll_bottom();
  }

  const input_row = document.createElement('div');
  input_row.className = 'chat-input-row';

  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'chat-input';
  input.maxLength = 256;
  input.placeholder = 'Type a message...';

  const send_btn = create_button('Send');

  function send_message() {
    const text = input.value.trim();
    if (!text) return;
    on_send(text);
    append_chat('You', text);
    input.value = '';
  }

  send_btn.addEventListener('click', send_message);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') send_message();
  });

  input_row.append(input, send_btn);
  section.appendChild(input_row);

  return { section, append_system, append_chat };
}

import { effect } from '@preact/signals-core';
import { page, is_multiplayer, room_code, connection_status } from '../state';
import { WsClient } from '../core/ws_client';
import { set_multiplayer_ws } from '../core/multiplayer';

const RELAY_URL = 'ws://localhost:9000';
const CODE_CHARS = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';

function gen_room_code(): string {
  return Array.from({ length: 4 }, () =>
    CODE_CHARS[Math.floor(Math.random() * CODE_CHARS.length)],
  ).join('');
}

export function create_lobby_screen(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'lobby';

  if (!room_code.value) {
    room_code.value = gen_room_code();
  }

  let current_ws = new WsClient(`${RELAY_URL}/room/${room_code.value}`);
  const peers: string[] = [];
  const ready_set = new Set<string>();
  let my_name = '';
  let is_ready = false;

  const layout = document.createElement('div');
  layout.className = 'lobby-layout';

  const main = document.createElement('div');
  main.className = 'lobby-main';

  const { section: room_section, join_input, join_btn } = build_room_code_section();
  const { section: player_section, update: update_players } = build_player_list();
  const { btn: ready_btn, set_ready: set_ready_btn } = build_ready_button();

  main.appendChild(room_section);
  main.appendChild(player_section);
  main.appendChild(ready_btn);

  const sidebar = document.createElement('div');
  sidebar.className = 'lobby-sidebar';
  const { section: chat_section, append_system, append_chat } = build_chat_section((text) => {
    current_ws.sendJson({ type: 'chat', text });
  });
  sidebar.appendChild(chat_section);

  layout.appendChild(main);
  layout.appendChild(sidebar);
  container.appendChild(layout);

  function check_all_ready() {
    if (peers.length < 2) return;
    const all = peers.every((p) => ready_set.has(p));
    if (all) {
      is_multiplayer.value = true;
      set_multiplayer_ws(current_ws);
      // Don't close ws — game.ts will use it
      dispose();
      page.value = 'game';
    }
  }

  function attach_handlers(ws: WsClient) {
    ws.onmessage = (msg) => {
      if (msg.type === 'presence') {
        peers.length = 0;
        peers.push(...msg.peers);
        // assign my_name on first presence (I'm the last in the list)
        if (!my_name && peers.length > 0) {
          my_name = peers[peers.length - 1];
        }
        // remove ready flags for peers that left
        for (const r of ready_set) {
          if (!peers.includes(r)) ready_set.delete(r);
        }
        update_players(peers, ready_set);
        check_all_ready();
      } else if (msg.type === 'ready') {
        ready_set.add(msg.name);
        update_players(peers, ready_set);
        append_system(`${msg.name} is ready`);
        check_all_ready();
      } else if (msg.type === 'chat') {
        append_chat(msg.from ?? 'Player', msg.text);
      } else if (msg.type === 'join') {
        append_system(`${msg.name} joined`);
      } else if (msg.type === 'leave') {
        ready_set.delete(msg.name);
        append_system(`${msg.name} left`);
        update_players(peers, ready_set);
      }
    };
  }

  function connect_to(code: string) {
    current_ws.close();
    room_code.value = code;
    const code_display = room_section.querySelector('.room-code');
    if (code_display) code_display.textContent = code;
    peers.length = 0;
    ready_set.clear();
    my_name = '';
    is_ready = false;
    set_ready_btn(false);
    update_players([], new Set());
    current_ws = new WsClient(`${RELAY_URL}/room/${code}`);
    attach_handlers(current_ws);
    current_ws.connect();
  }

  ready_btn.addEventListener('click', () => {
    if (is_ready) return;
    is_ready = true;
    set_ready_btn(true);
    if (my_name) {
      ready_set.add(my_name);
      current_ws.sendJson({ type: 'ready', name: my_name });
      update_players(peers, ready_set);
      check_all_ready();
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

  const copy_btn = document.createElement('button');
  copy_btn.className = 'btn';
  copy_btn.textContent = 'Copy';
  copy_btn.addEventListener('click', () => {
    if (room_code.value) {
      navigator.clipboard.writeText(room_code.value).catch(() => {});
    }
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

function build_player_list(): {
  section: HTMLElement;
  update: (peers: string[], ready: Set<string>) => void;
} {
  const section = document.createElement('div');
  section.className = 'lobby-section player-list';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'PLAYERS';
  section.appendChild(label);

  const cards_container = document.createElement('div');
  section.appendChild(cards_container);

  function render(peers: string[], ready: Set<string>) {
    cards_container.innerHTML = '';
    const slots = Math.max(peers.length, 1);
    for (let i = 0; i < slots; i++) {
      const card = document.createElement('div');
      card.className = 'player-card glass';
      if (peers[i]) {
        const r = ready.has(peers[i]);
        const status_class = r ? 'player-status player-ready' : 'player-status';
        const status_text = r ? '✓ READY' : 'Not Ready';
        card.innerHTML = `<span class="player-name">${peers[i]}</span><span class="${status_class}">${status_text}</span>`;
      } else {
        card.innerHTML = `<span class="player-name">—</span><span class="player-status">Waiting...</span>`;
      }
      cards_container.appendChild(card);
    }
  }

  render([], new Set());

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
      btn.textContent = 'WAITING...';
      btn.disabled = true;
      btn.classList.add('ready-btn-active');
    } else {
      btn.textContent = 'READY';
      btn.disabled = false;
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

  const send_btn = document.createElement('button');
  send_btn.className = 'btn';
  send_btn.textContent = 'Send';

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

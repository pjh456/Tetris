import { effect } from '@preact/signals-core';
import { page, is_multiplayer, room_code, connection_status } from '../state';
import { WsClient } from '../core/ws_client';

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

  const layout = document.createElement('div');
  layout.className = 'lobby-layout';

  const main = document.createElement('div');
  main.className = 'lobby-main';

  const { section: room_section, join_input, join_btn } = build_room_code_section();
  const { section: player_section, update: update_players } = build_player_list();
  main.appendChild(room_section);
  main.appendChild(player_section);
  main.appendChild(build_ready_button());

  const sidebar = document.createElement('div');
  sidebar.className = 'lobby-sidebar';
  const { section: chat_section, append_system, append_chat } = build_chat_section((text) => {
    current_ws.sendJson({ type: 'chat', text });
  });
  sidebar.appendChild(chat_section);

  layout.appendChild(main);
  layout.appendChild(sidebar);
  container.appendChild(layout);

  function attach_handlers(ws: WsClient) {
    ws.onmessage = (msg) => {
      if (msg.type === 'presence') {
        peers.length = 0;
        peers.push(...msg.peers);
        update_players(peers);
      } else if (msg.type === 'chat') {
        append_chat(msg.from ?? 'Player', msg.text);
      } else if (msg.type === 'join') {
        append_system(`${msg.name} joined`);
      } else if (msg.type === 'leave') {
        append_system(`${msg.name} left`);
      }
    };
  }

  function connect_to(code: string) {
    current_ws.close();
    room_code.value = code;
    const code_display = room_section.querySelector('.room-code');
    if (code_display) code_display.textContent = code;
    peers.length = 0;
    update_players([]);
    current_ws = new WsClient(`${RELAY_URL}/room/${code}`);
    attach_handlers(current_ws);
    current_ws.connect();
  }

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
    current_ws.close();
  }

  container.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      cleanup();
      page.value = 'home';
    }
  });

  container.dataset['cleanup'] = 'pending';
  // Expose cleanup so main.ts can call it on page change
  (container as HTMLElement & { _cleanup?: () => void })._cleanup = cleanup;

  return container;
}

function build_room_code_section(): { section: HTMLElement; join_input: HTMLInputElement; join_btn: HTMLButtonElement } {
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
  update: (peers: string[]) => void;
} {
  const section = document.createElement('div');
  section.className = 'lobby-section player-list';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'PLAYERS';
  section.appendChild(label);

  const cards_container = document.createElement('div');
  section.appendChild(cards_container);

  function render(peers: string[]) {
    cards_container.innerHTML = '';
    const slots = Math.max(peers.length, 1);
    for (let i = 0; i < slots; i++) {
      const card = document.createElement('div');
      card.className = 'player-card glass';
      if (peers[i]) {
        card.innerHTML = `<span class="player-name">${peers[i]}</span><span class="player-status ready">Connected</span>`;
      } else {
        card.innerHTML = `<span class="player-name">—</span><span class="player-status">Waiting...</span>`;
      }
      cards_container.appendChild(card);
    }
  }

  render([]);

  return { section, update: render };
}

function build_ready_button(): HTMLElement {
  const btn = document.createElement('button');
  btn.className = 'btn ready-btn';
  btn.textContent = 'READY';
  btn.addEventListener('click', () => {
    is_multiplayer.value = true;
    page.value = 'game';
  });
  return btn;
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

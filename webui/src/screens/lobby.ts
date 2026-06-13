import { page, is_multiplayer, room_code, connection_status } from '../state';

export function create_lobby_screen(): HTMLElement {
  const container = document.createElement('div');
  container.className = 'lobby';

  const layout = document.createElement('div');
  layout.className = 'lobby-layout';

  const main = document.createElement('div');
  main.className = 'lobby-main';

  main.appendChild(build_room_code_section());
  main.appendChild(build_player_list());
  main.appendChild(build_ready_button());

  const sidebar = document.createElement('div');
  sidebar.className = 'lobby-sidebar';
  sidebar.appendChild(build_chat_section());

  layout.appendChild(main);
  layout.appendChild(sidebar);
  container.appendChild(layout);

  container.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      page.value = 'home';
    }
  });

  return container;
}

function build_room_code_section(): HTMLElement {
  const section = document.createElement('div');
  section.className = 'lobby-section';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'ROOM CODE';

  const code_display = document.createElement('div');
  code_display.className = 'room-code';
  code_display.textContent = room_code.value || '----';

  const copy_btn = document.createElement('button');
  copy_btn.className = 'btn';
  copy_btn.textContent = 'Copy';
  copy_btn.addEventListener('click', () => {
    if (room_code.value) {
      navigator.clipboard.writeText(room_code.value).catch(() => {});
    }
  });

  const status = document.createElement('span');
  status.className = `status-dot status-${connection_status.value}`;

  section.append(label, code_display, status, copy_btn);
  return section;
}

function build_player_list(): HTMLElement {
  const section = document.createElement('div');
  section.className = 'lobby-section player-list';

  const label = document.createElement('div');
  label.className = 'lobby-label';
  label.textContent = 'PLAYERS';

  section.appendChild(label);

  for (let i = 0; i < 4; i++) {
    const card = document.createElement('div');
    card.className = 'player-card glass';
    card.innerHTML = `
            <span class="player-name">Player ${i + 1}</span>
            <span class="player-status">Not Ready</span>
        `;
    section.appendChild(card);
  }

  return section;
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

function build_chat_section(): HTMLElement {
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
    const el = document.createElement('div');
    el.className = 'chat-message';
    el.textContent = text;
    const empty = messages.querySelector('.chat-empty');
    if (empty) empty.remove();
    messages.appendChild(el);
    messages.scrollTop = messages.scrollHeight;
    input.value = '';
  }

  send_btn.addEventListener('click', send_message);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      send_message();
    }
  });

  input_row.append(input, send_btn);
  section.appendChild(input_row);
  return section;
}

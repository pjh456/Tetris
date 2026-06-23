import { create_button } from '../core/dom';
import { connection_status } from '../state';
import { get_server_url, set_server_url, set_offline } from '../core/server_config';
import { is_tauri, lan_discover } from '../core/lan_discovery';

const PROBE_TIMEOUT_MS = 5000;

export type ConnectDialogOpts = {
  on_connected: () => void;
  on_offline: () => void;
};

export function create_connect_dialog(opts: ConnectDialogOpts): () => void {
  let probe: WebSocket | null = null;
  let probe_timer: ReturnType<typeof setTimeout> | null = null;

  const overlay = document.createElement('div');
  overlay.className = 'connect-overlay';

  const panel = document.createElement('div');
  panel.className = 'connect-panel glass';
  overlay.appendChild(panel);

  function clear_probe() {
    if (probe_timer) {
      clearTimeout(probe_timer);
      probe_timer = null;
    }
    if (probe) {
      probe.onopen = null;
      probe.onerror = null;
      probe.onclose = null;
      probe.close();
      probe = null;
    }
  }

  function dispose() {
    clear_probe();
    overlay.remove();
  }

  function probe_connect(url: string, on_error: (msg: string) => void) {
    clear_probe();
    connection_status.value = 'connecting';
    let socket: WebSocket;
    try {
      socket = new WebSocket(url);
    } catch {
      connection_status.value = 'offline';
      on_error('无效的服务器地址');
      return;
    }
    probe = socket;
    probe_timer = setTimeout(() => {
      clear_probe();
      connection_status.value = 'offline';
      on_error('连接超时，请检查地址或服务器状态');
    }, PROBE_TIMEOUT_MS);
    socket.onopen = () => {
      clear_probe();
      connection_status.value = 'online';
      dispose();
      opts.on_connected();
    };
    socket.onerror = () => {
      clear_probe();
      connection_status.value = 'offline';
      on_error('无法连接到服务器');
    };
  }

  // ── Primary view: address input + 3 actions ────────────────────────────
  const primary = document.createElement('div');
  primary.className = 'connect-view';

  const title = document.createElement('div');
  title.className = 'connect-title';
  title.textContent = '连接到服务器';

  const desc = document.createElement('div');
  desc.className = 'connect-desc';
  desc.textContent = '输入 relay 服务器地址（host:port 或 ws://...），或选择离线模式。';

  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'connect-input';
  input.placeholder = 'ws://localhost:9000';
  input.value = get_server_url();

  const error_el = document.createElement('div');
  error_el.className = 'connect-error';

  const buttons = document.createElement('div');
  buttons.className = 'connect-buttons';

  const connect_btn = create_button('连接到服务器', { ariaLabel: '连接到服务器' });
  const lan_btn = create_button('局域网发现', { ariaLabel: '局域网发现' });
  const offline_btn = create_button('进入离线模式', { ariaLabel: '进入离线模式' });
  buttons.append(connect_btn, lan_btn, offline_btn);

  primary.append(title, desc, input, error_el, buttons);

  function connect_from_input() {
    error_el.textContent = '';
    const url = set_server_url(input.value);
    input.value = url;
    connect_btn.disabled = true;
    connect_btn.textContent = '连接中…';
    probe_connect(url, (msg) => {
      error_el.textContent = msg;
      connect_btn.disabled = false;
      connect_btn.textContent = '连接到服务器';
    });
  }

  // ── Secondary view: LAN discovery ──────────────────────────────────────
  const secondary = document.createElement('div');
  secondary.className = 'connect-view';
  secondary.style.display = 'none';

  const lan_title = document.createElement('div');
  lan_title.className = 'connect-title';
  lan_title.textContent = '局域网发现';

  const spinner = document.createElement('div');
  spinner.className = 'connect-spinner';

  const lan_list = document.createElement('div');
  lan_list.className = 'connect-lan-list';

  const lan_error = document.createElement('div');
  lan_error.className = 'connect-error';

  const back_btn = create_button('返回', { ariaLabel: '返回连接界面' });

  secondary.append(lan_title, spinner, lan_list, lan_error, back_btn);

  function set_lan_message(text: string) {
    lan_list.innerHTML = '';
    const empty = document.createElement('div');
    empty.className = 'connect-lan-empty';
    empty.textContent = text;
    lan_list.appendChild(empty);
  }

  async function run_scan() {
    lan_error.textContent = '';
    if (!is_tauri()) {
      spinner.style.display = 'none';
      set_lan_message('仅桌面版支持局域网发现');
      return;
    }
    spinner.style.display = 'block';
    set_lan_message('扫描中…');
    const relays = await lan_discover();
    spinner.style.display = 'none';
    if (relays.length === 0) {
      set_lan_message('未发现局域网服务器');
      return;
    }
    lan_list.innerHTML = '';
    for (const relay of relays) {
      const item = create_button(`${relay.label}  ·  ${relay.ws_url}`, {
        ariaLabel: `连接到 ${relay.label}`,
        onClick: () => {
          set_server_url(relay.ws_url);
          probe_connect(relay.ws_url, (msg) => {
            lan_error.textContent = msg;
          });
        },
      });
      item.classList.add('connect-lan-item');
      lan_list.appendChild(item);
    }
  }

  panel.append(primary, secondary);

  function show_primary() {
    secondary.style.display = 'none';
    primary.style.display = 'flex';
  }

  function show_secondary() {
    primary.style.display = 'none';
    secondary.style.display = 'flex';
    void run_scan();
  }

  connect_btn.addEventListener('click', connect_from_input);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') connect_from_input();
  });
  lan_btn.addEventListener('click', show_secondary);
  back_btn.addEventListener('click', show_primary);
  offline_btn.addEventListener('click', () => {
    set_offline(true);
    dispose();
    opts.on_offline();
  });

  show_primary();
  document.body.appendChild(overlay);
  input.focus();

  return dispose;
}

import { create_button } from '../core/dom';
import { connection_status } from '../state';
import { get_server_url, set_server_url, set_offline } from '../core/server_config';

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

  // ── Secondary view: LAN discovery skeleton (no real scan, PLAN-23) ─────
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
  const lan_placeholder = document.createElement('div');
  lan_placeholder.className = 'connect-lan-empty';
  lan_placeholder.textContent = '局域网发现待实现（PLAN-23）';
  lan_list.appendChild(lan_placeholder);

  const back_btn = create_button('返回', { ariaLabel: '返回连接界面' });

  secondary.append(lan_title, spinner, lan_list, back_btn);

  panel.append(primary, secondary);

  function show_primary() {
    secondary.style.display = 'none';
    primary.style.display = 'flex';
  }

  function show_secondary() {
    primary.style.display = 'none';
    secondary.style.display = 'flex';
  }

  function try_connect() {
    clear_probe();
    error_el.textContent = '';
    const url = set_server_url(input.value);
    input.value = url;
    connect_btn.disabled = true;
    connect_btn.textContent = '连接中…';
    connection_status.value = 'connecting';

    let socket: WebSocket;
    try {
      socket = new WebSocket(url);
    } catch {
      fail('无效的服务器地址');
      return;
    }
    probe = socket;

    probe_timer = setTimeout(() => {
      fail('连接超时，请检查地址或服务器状态');
    }, PROBE_TIMEOUT_MS);

    socket.onopen = () => {
      clear_probe();
      connection_status.value = 'online';
      dispose();
      opts.on_connected();
    };
    socket.onerror = () => {
      fail('无法连接到服务器');
    };
  }

  function fail(message: string) {
    clear_probe();
    connection_status.value = 'offline';
    error_el.textContent = message;
    connect_btn.disabled = false;
    connect_btn.textContent = '连接到服务器';
  }

  connect_btn.addEventListener('click', try_connect);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') try_connect();
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

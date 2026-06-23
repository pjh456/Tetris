import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { connection_status } from '../state';
import type { ConnectionState } from '../state';
import type { MultiplayerEvent } from './wasm';

const HEARTBEAT_MS = 1000;
const DISCONNECT_TIMEOUT_MS = 3000;
const MAX_RECONNECT_ATTEMPTS = 3;

export class WsClient {
  private socket: WebSocket | null = null;
  private url: string;
  private wasm: WebTetris | null;
  private heartbeat_timer: ReturnType<typeof setInterval> | null = null;
  private reconnect_timeout: ReturnType<typeof setTimeout> | null = null;
  private reconnect_timer: ReturnType<typeof setTimeout> | null = null;
  private last_pong_time = 0;
  private reconnect_attempt = 0;
  private _resume_token: string | null = null;
  private _socket_id: string | null = null;
  private message_buffer: Uint8Array[] = [];
  private was_reconnecting = false;

  onpacket: ((event: MultiplayerEvent | null) => void) | null = null;
  onopen: (() => void) | null = null;

  constructor(url: string, wasm: WebTetris | null = null) {
    this.url = url;
    this.wasm = wasm;
  }

  connect() {
    connection_status.value = 'connecting';
    this.create_socket();
  }

  private create_socket() {
    if (this.socket) {
      this.socket.onopen = null;
      this.socket.onmessage = null;
      this.socket.onclose = null;
      this.socket.onerror = null;
      this.socket.close();
    }
    this.socket = new WebSocket(this.url);
    this.socket.binaryType = 'arraybuffer';

    this.socket.onopen = () => {
      if (this.reconnect_timeout) {
        clearTimeout(this.reconnect_timeout);
        this.reconnect_timeout = null;
      }
      const should_resume = this.was_reconnecting && this.can_resume();
      connection_status.value = should_resume ? 'resyncing' : 'online';
      this.reconnect_attempt = 0;
      this.last_pong_time = Date.now();
      this.start_heartbeat();
      // Explicit handshake: the first packet always declares intent. A non-empty
      // resume_token = resume attempt; empty = fresh join. Sent raw (not buffered)
      // so the server resolves intent before any other packet.
      this.send_connect_packet(should_resume ? (this._resume_token ?? '') : '');
      if (should_resume) {
        this.send_resume_packets();
      } else {
        this.flush_buffer();
      }
      this.onopen?.();
    };

    this.socket.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      this.last_pong_time = Date.now();
      const data = new Uint8Array(event.data);
      if (this.wasm) {
        try {
          const parsed = this.wasm.parse_packet(data) as MultiplayerEvent | null;
          if (parsed?.kind === 'server_accept' && typeof parsed.resume_token === 'string') {
            const socket_id = typeof parsed.player_id === 'number' ? String(parsed.player_id) : '';
            this.set_resume_token(parsed.resume_token, socket_id);
          }
          this.onpacket?.(parsed);
        } catch {
          // skip corrupt packet
        }
      }
    };

    this.socket.onclose = () => {
      this.stop_heartbeat();
      if (this.should_attempt_reconnect()) {
        this.begin_reconnect();
      }
    };

    this.socket.onerror = () => {
      if (this.should_attempt_reconnect()) {
        this.begin_reconnect();
      }
    };
  }

  private should_attempt_reconnect(): boolean {
    const status = connection_status.value as ConnectionState;
    // 'connecting' is included so a failed INITIAL connection degrades to
    // disconnected (with retries) instead of hanging forever.
    return (
      status === 'online' ||
      status === 'slow' ||
      status === 'reconnecting' ||
      status === 'connecting'
    );
  }

  private start_heartbeat() {
    this.stop_heartbeat();
    this.heartbeat_timer = setInterval(() => {
      if (this.socket?.readyState === WebSocket.OPEN) {
        this.socket.send(new Uint8Array(0));
      }
      // Only adjust slow/online while actually connected. Never override
      // resyncing/reconnecting/connecting/disconnected — doing so previously
      // interrupted the reconnect/resync flow.
      const status = connection_status.value as ConnectionState;
      if (status !== 'online' && status !== 'slow') return;
      const elapsed = Date.now() - this.last_pong_time;
      if (elapsed > 500) {
        connection_status.value = 'slow';
      } else if (status === 'slow') {
        connection_status.value = 'online';
      }
    }, HEARTBEAT_MS);
  }

  private stop_heartbeat() {
    if (this.heartbeat_timer) {
      clearInterval(this.heartbeat_timer);
      this.heartbeat_timer = null;
    }
  }

  private begin_reconnect() {
    if (connection_status.value === 'reconnecting') return;
    if (this.reconnect_attempt >= MAX_RECONNECT_ATTEMPTS) {
      connection_status.value = 'disconnected';
      return;
    }
    connection_status.value = 'reconnecting';
    this.was_reconnecting = true;
    this.reconnect_timeout = setTimeout(() => {
      connection_status.value = 'disconnected';
    }, DISCONNECT_TIMEOUT_MS);
    this.attempt_reconnect();
  }

  private attempt_reconnect() {
    const delay = Math.min(1000 * Math.pow(2, this.reconnect_attempt), 4000);
    this.reconnect_timer = setTimeout(() => {
      this.reconnect_attempt += 1;
      this.create_socket();
    }, delay);
  }

  send(data: Uint8Array) {
    if (connection_status.value === 'resyncing') {
      if (this.message_buffer.length < 256) this.message_buffer.push(data.slice());
      return;
    }
    if (this.socket?.readyState === WebSocket.OPEN) {
      this.socket.send(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
    } else if (this.message_buffer.length < 256) {
      this.message_buffer.push(data.slice());
    }
  }

  is_open(): boolean {
    return this.socket?.readyState === WebSocket.OPEN;
  }

  private flush_buffer() {
    const buf = this.message_buffer;
    this.message_buffer = [];
    for (const data of buf) {
      this.send(data);
    }
  }

  set_resume_token(token: string, socket_id: string) {
    this._resume_token = token;
    this._socket_id = socket_id;
  }

  can_resume(): boolean {
    return this._resume_token !== null && this._socket_id !== null;
  }

  private send_connect_packet(token: string) {
    if (!this.wasm) return;
    const pkt = this.wasm.make_connect_packet(token);
    this.socket?.send(pkt.buffer.slice(pkt.byteOffset, pkt.byteOffset + pkt.byteLength));
  }

  private send_resume_packets() {
    if (!this.wasm || !this._resume_token || !this._socket_id) return;
    const resume_packet = this.wasm.make_resume_packet(this._socket_id, this._resume_token);
    const reconnect_packet = this.wasm.make_reconnect_packet();
    this.socket?.send(
      resume_packet.buffer.slice(
        resume_packet.byteOffset,
        resume_packet.byteOffset + resume_packet.byteLength,
      ),
    );
    this.socket?.send(
      reconnect_packet.buffer.slice(
        reconnect_packet.byteOffset,
        reconnect_packet.byteOffset + reconnect_packet.byteLength,
      ),
    );
  }

  finish_resync() {
    if (connection_status.value === 'resyncing') {
      connection_status.value = 'online';
      this.was_reconnecting = false;
      this.flush_buffer();
    }
  }

  /**
   * Manual reconnect entry point (for the connection UI after auto-reconnect
   * exhausts or the user is disconnected). Resets the attempt counter and
   * re-opens the socket from a clean state.
   */
  reconnect() {
    if (this.reconnect_timeout) {
      clearTimeout(this.reconnect_timeout);
      this.reconnect_timeout = null;
    }
    if (this.reconnect_timer) {
      clearTimeout(this.reconnect_timer);
      this.reconnect_timer = null;
    }
    this.reconnect_attempt = 0;
    connection_status.value = 'connecting';
    this.create_socket();
  }

  close() {
    this.stop_heartbeat();
    if (this.reconnect_timeout) {
      clearTimeout(this.reconnect_timeout);
    }
    if (this.reconnect_timer) {
      clearTimeout(this.reconnect_timer);
    }
    this.socket?.close();
    this.reconnect_attempt = 0;
    this.was_reconnecting = false;
    connection_status.value = 'offline';
  }
}

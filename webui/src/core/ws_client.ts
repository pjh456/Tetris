import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { connection_status } from '../state';
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
    this.socket = new WebSocket(this.url);
    this.socket.binaryType = 'arraybuffer';

    this.socket.onopen = () => {
      if (this.reconnect_timeout) {
        clearTimeout(this.reconnect_timeout);
        this.reconnect_timeout = null;
      }
      connection_status.value = 'online';
      this.reconnect_attempt = 0;
      this.last_pong_time = Date.now();
      this.start_heartbeat();
      this.flush_buffer();
      this.onopen?.();
    };

    this.socket.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      const data = new Uint8Array(event.data);
      if (this.wasm) {
        const parsed = this.wasm.parse_packet(data) as MultiplayerEvent | null;
        this.onpacket?.(parsed);
      }
    };

    this.socket.onclose = () => {
      this.stop_heartbeat();
      if (connection_status.value === 'online' || connection_status.value === 'slow' || connection_status.value === 'reconnecting') {
        this.begin_reconnect();
      }
    };

    this.socket.onerror = () => {
      if (connection_status.value === 'online' || connection_status.value === 'slow' || connection_status.value === 'reconnecting') {
        this.begin_reconnect();
      }
    };
  }

  private start_heartbeat() {
    this.stop_heartbeat();
    this.heartbeat_timer = setInterval(() => {
      if (this.socket?.readyState === WebSocket.OPEN) {
        this.socket.send(new Uint8Array(0));
        this.last_pong_time = Date.now();
      }
      const elapsed = Date.now() - this.last_pong_time;
      if (elapsed > 500) {
        connection_status.value = 'slow';
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
    if (this.reconnect_attempt >= MAX_RECONNECT_ATTEMPTS) {
      connection_status.value = 'disconnected';
      return;
    }
    connection_status.value = 'reconnecting';
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
    if (this.socket?.readyState === WebSocket.OPEN) {
      this.socket.send(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
    } else {
      this.message_buffer.push(data);
    }
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

  close() {
    this.stop_heartbeat();
    if (this.reconnect_timeout) {
      clearTimeout(this.reconnect_timeout);
    }
    if (this.reconnect_timer) {
      clearTimeout(this.reconnect_timer);
    }
    this.socket?.close();
    connection_status.value = 'offline';
  }
}

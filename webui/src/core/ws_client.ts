import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { connection_status } from '../state';

export type RelayMessage =
  | { type: 'presence'; peers: string[] }
  | { type: 'chat'; text: string; from?: string }
  | { type: 'join'; name: string }
  | { type: 'leave'; name: string };

export class WsClient {
  private socket: WebSocket | null = null;
  private url: string;
  private wasm: WebTetris | null;
  onmessage: ((msg: RelayMessage) => void) | null = null;

  constructor(url: string, wasm: WebTetris | null = null) {
    this.url = url;
    this.wasm = wasm;
  }

  connect() {
    connection_status.value = 'connecting';
    this.socket = new WebSocket(this.url);
    this.socket.binaryType = 'arraybuffer';

    this.socket.onopen = () => {
      connection_status.value = 'connected';
    };

    this.socket.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      const data = new Uint8Array(event.data);
      // Try JSON relay message first; fall back to wasm binary protocol
      try {
        const text = new TextDecoder().decode(data);
        const msg = JSON.parse(text) as RelayMessage;
        if (this.onmessage) this.onmessage(msg);
        return;
      } catch {
        // not a JSON relay message
      }
      if (this.wasm) this.wasm.parse_packet(data);
    };

    this.socket.onclose = () => {
      connection_status.value = 'disconnected';
    };

    this.socket.onerror = () => {
      connection_status.value = 'disconnected';
    };
  }

  sendJson(msg: RelayMessage) {
    const bytes = new TextEncoder().encode(JSON.stringify(msg));
    this.send(bytes);
  }

  send(data: Uint8Array) {
    if (this.socket?.readyState === WebSocket.OPEN) {
      this.socket.send(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
    }
  }

  close() {
    this.socket?.close();
  }
}

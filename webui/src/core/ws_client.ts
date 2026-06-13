import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { connection_status } from '../state';

export class WsClient {
  private socket: WebSocket | null = null;
  private url: string;
  private wasm: WebTetris;

  constructor(url: string, wasm: WebTetris) {
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
      this.wasm.parse_packet(data);
    };

    this.socket.onclose = () => {
      connection_status.value = 'disconnected';
    };

    this.socket.onerror = () => {
      connection_status.value = 'disconnected';
    };
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

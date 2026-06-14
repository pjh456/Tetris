import type { WebTetris } from '../../wasm/tetris_wasm.js';
import { connection_status } from '../state';
import type { MultiplayerEvent } from './wasm';

export class WsClient {
  private socket: WebSocket | null = null;
  private url: string;
  private wasm: WebTetris | null;
  onpacket: ((event: MultiplayerEvent | null) => void) | null = null;
  onopen: (() => void) | null = null;

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
      this.onopen?.();
    };

    this.socket.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      const data = new Uint8Array(event.data);
      if (this.wasm) {
        const parsed = this.wasm.parse_packet(data) as MultiplayerEvent | null;
        this.onpacket?.(parsed);
      } else {
        this.onpacket?.(null);
      }
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

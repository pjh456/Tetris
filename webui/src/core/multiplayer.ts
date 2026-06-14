import type { WsClient } from './ws_client';

let _ws: WsClient | null = null;

export function set_multiplayer_ws(ws: WsClient | null) {
  _ws = ws;
}

export function get_multiplayer_ws(): WsClient | null {
  return _ws;
}

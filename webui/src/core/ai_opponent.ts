import { make_add_bot_packet } from './wasm';
import type { WsClient } from './ws_client';

const DEFAULT_AI_TEMP = 0;

/**
 * Summon a networked AI opponent (bot) via the relay: sends an AddBot request
 * over the room websocket. The bot runs server-side as a normal player.
 * (Local single-player AI was removed — see the WatchAI screen for AI demos.)
 */
export function add_multiplayer_ai_opponent(ws: WsClient, temperature = DEFAULT_AI_TEMP): boolean {
  if (!ws.is_open()) {
    return false;
  }
  ws.send(make_add_bot_packet(temperature));
  return true;
}

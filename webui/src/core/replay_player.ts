export class OpponentReplayPlayer {
  push_replay(_player_id: number, _events: unknown[]) {
    // ServerReplay events are handled directly by WASM parse_packet.
  }

  apply_tick(_tick: number) {
    // Tick tracking for future event scheduling.
  }
}

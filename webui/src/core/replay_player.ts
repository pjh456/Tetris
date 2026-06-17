export class OpponentReplayPlayer {
  readonly player_id: number;
  private pending_events: unknown[] = [];
  private last_tick = 0;

  constructor(player_id: number) {
    this.player_id = player_id;
  }

  push_replay(player_id: number, events: unknown[]) {
    if (player_id !== this.player_id) return;
    this.pending_events.push(...events);
  }

  apply_tick(tick: number) {
    this.last_tick = Math.max(this.last_tick, tick);
    this.pending_events = this.pending_events.filter((event) => {
      if (!event || typeof event !== 'object') return false;
      const tick_value = (event as { tick?: unknown }).tick;
      if (typeof tick_value === 'number') return tick_value > this.last_tick;
      if (tick_value && typeof tick_value === 'object') {
        const raw_tick = (tick_value as { '0'?: unknown })['0'];
        return typeof raw_tick === 'number' && raw_tick > this.last_tick;
      }
      return false;
    });
  }

  pending_count(): number {
    return this.pending_events.length;
  }
}

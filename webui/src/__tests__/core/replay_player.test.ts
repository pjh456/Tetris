import { describe, expect, it } from 'vitest';

import { OpponentReplayPlayer } from '../../core/replay_player';

describe('OpponentReplayPlayer', () => {
  it('tracks only matching player replay events', () => {
    const player = new OpponentReplayPlayer(2);

    player.push_replay(1, [{ tick: 1 }]);
    player.push_replay(2, [{ tick: 1 }, { tick: 3 }]);

    expect(player.pending_count()).toBe(2);
  });

  it('drops events at or before applied tick', () => {
    const player = new OpponentReplayPlayer(2);

    player.push_replay(2, [{ tick: 1 }, { tick: 3 }]);
    player.apply_tick(1);

    expect(player.pending_count()).toBe(1);
  });
});

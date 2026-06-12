import { describe, it, expect } from 'vitest';
import { Actions } from '../../game/actions';

describe('Actions', () => {
  it('defines MoveLeft as 0', () => {
    expect(Actions.MoveLeft).toBe(0);
  });

  it('defines MoveRight as 1', () => {
    expect(Actions.MoveRight).toBe(1);
  });

  it('defines HardDrop as 3', () => {
    expect(Actions.HardDrop).toBe(3);
  });

  it('defines Hold as 6', () => {
    expect(Actions.Hold).toBe(6);
  });

  it('has 7 unique action values', () => {
    const values = Object.values(Actions);
    expect(values).toHaveLength(7);
    expect(new Set(values).size).toBe(7);
  });

  it('all values are in 0-6 range', () => {
    Object.values(Actions).forEach((v) => {
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThanOrEqual(6);
    });
  });
});

import { signal, computed } from '@preact/signals-core';
import { load_settings, type Settings } from './core/settings_store';

export type Page =
  | 'home'
  | 'game'
  | 'settings'
  | 'gameover'
  | 'lobby'
  | 'leaderboard'
  | 'spectator';

export const page = signal<Page>('home');
export const is_multiplayer = signal(false);
export const room_code = signal<string | null>(null);
export type ConnectionState =
  | 'offline'
  | 'connecting'
  | 'online'
  | 'slow'
  | 'reconnecting'
  | 'disconnected'
  | 'resyncing';

export const connection_status = signal<ConnectionState>('offline');

export const score = signal(0);
export const level = signal(1);
export const combo = signal(0);
export const b2b_count = signal(0);
export const lines = signal(0);
export const tspin_active = signal(false);
export const all_clear_active = signal(false);

export const formatted_score = computed(() => score.value.toLocaleString('en-US'));

export const settings = signal<Settings>(load_settings());

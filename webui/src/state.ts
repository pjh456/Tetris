import { signal, computed } from '@preact/signals-core';
import { load_settings, type Settings } from './core/settings_store';

export type Page = 'home' | 'game' | 'settings' | 'gameover';

export const page = signal<Page>('home');

export const score = signal(0);
export const level = signal(1);
export const combo = signal(0);
export const b2b_count = signal(0);
export const lines = signal(0);
export const tspin_active = signal(false);
export const all_clear_active = signal(false);

export const formatted_score = computed(() => score.value.toLocaleString('en-US'));

export const settings = signal<Settings>(load_settings());

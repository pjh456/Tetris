import { server_offline } from '../state';

const STORAGE_KEY = 'tetris_server_url';
const DEFAULT_SERVER_URL = 'ws://localhost:9000';

export function normalize_server_url(raw: string): string {
  const trimmed = raw.trim().replace(/\/+$/, '');
  if (!trimmed) return DEFAULT_SERVER_URL;
  if (/^wss?:\/\//i.test(trimmed)) return trimmed;
  if (/^https:\/\//i.test(trimmed)) return trimmed.replace(/^https/i, 'wss');
  if (/^http:\/\//i.test(trimmed)) return trimmed.replace(/^http/i, 'ws');
  return `ws://${trimmed}`;
}

export function get_server_url(): string {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? normalize_server_url(raw) : DEFAULT_SERVER_URL;
  } catch {
    return DEFAULT_SERVER_URL;
  }
}

export function set_server_url(raw: string): string {
  const normalized = normalize_server_url(raw);
  try {
    localStorage.setItem(STORAGE_KEY, normalized);
  } catch {
    // quota exceeded or privacy mode — silently ignore
  }
  return normalized;
}

export function is_offline(): boolean {
  return server_offline.value;
}

export function set_offline(value: boolean): void {
  server_offline.value = value;
}

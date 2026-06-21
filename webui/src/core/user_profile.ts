export interface UserProfile {
  name: string;
  avatar?: string;
}

const STORAGE_KEY = 'tetris-user-profile';
const DEFAULT_NAME = 'Guest';
const MAX_NAME_LEN = 16;

function clean_name(value: unknown): string {
  if (typeof value !== 'string') return DEFAULT_NAME;
  const trimmed = value.trim();
  return trimmed ? trimmed.slice(0, MAX_NAME_LEN) : DEFAULT_NAME;
}

export function load_profile(): UserProfile {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { name: DEFAULT_NAME };
    const parsed = JSON.parse(raw);
    const name = clean_name(parsed?.name);
    const avatar = typeof parsed?.avatar === 'string' ? parsed.avatar : undefined;
    return avatar ? { name, avatar } : { name };
  } catch {
    return { name: DEFAULT_NAME };
  }
}

export function save_profile(profile: UserProfile): void {
  const clean: UserProfile = { name: clean_name(profile.name) };
  if (profile.avatar) clean.avatar = profile.avatar;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(clean));
  } catch {
    // quota exceeded or privacy mode — silently ignore
  }
}

export function get_display_name(): string {
  return load_profile().name;
}

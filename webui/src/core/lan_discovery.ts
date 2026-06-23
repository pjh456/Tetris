export type RelayInfo = {
  label: string;
  ws_url: string;
  version: string;
};

type TauriInvoke = (cmd: string, args?: unknown) => Promise<unknown>;

function get_invoke(): TauriInvoke | null {
  if (typeof window === 'undefined') return null;
  const w = window as unknown as { __TAURI_INTERNALS__?: { invoke?: TauriInvoke } };
  return w.__TAURI_INTERNALS__?.invoke ?? null;
}

export function is_tauri(): boolean {
  return get_invoke() !== null;
}

function is_relay_info(v: unknown): v is RelayInfo {
  if (typeof v !== 'object' || v === null) return false;
  const r = v as Record<string, unknown>;
  return (
    typeof r.label === 'string' && typeof r.ws_url === 'string' && typeof r.version === 'string'
  );
}

export async function lan_discover(): Promise<RelayInfo[]> {
  const invoke = get_invoke();
  if (!invoke) return [];
  try {
    const result = await invoke('lan_discover');
    if (!Array.isArray(result)) return [];
    return result.filter(is_relay_info);
  } catch {
    return [];
  }
}

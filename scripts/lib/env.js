import { join, dirname } from 'node:path';

const __dirname = dirname(import.meta.filename);

export const ROOT = join(__dirname, '..', '..');
export const CRATES = join(ROOT, 'crates');
export const WEBUI = join(ROOT, 'webui');
export const SCRIPTS = join(ROOT, 'scripts');
export const PKG = join(CRATES, 'tetris-wasm', 'pkg');
export const WASM_OUT = join(WEBUI, 'wasm');
export const CLI_CRATE = join(CRATES, 'tetris-cli');
export const WASM_CRATE = join(CRATES, 'tetris-wasm');
export const CORE_CRATE = join(CRATES, 'tetris-core');
export const NET_CRATE = join(CRATES, 'tetris-net');
export const AI_CRATE = join(CRATES, 'tetris-ai');

import { wasmPackBuild } from './lib/build.js';
import { WEBUI } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('wasm-pack build', wasmPackBuild);
await step('vite build (webui)', () => run('npx.cmd', ['vite', 'build'], { cwd: WEBUI }));
await step('tauri bundle', () =>
    run('npx.cmd', ['@tauri-apps/cli', 'build', '--config', 'crates/tetris-tauri/tauri.conf.json']),
);

import { cargoBuildAll, wasmPackBuild, npmInstallWebui } from './lib/build.js';
import { WEBUI } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('cargo build --workspace --release', () => cargoBuildAll(true));
await step('wasm-pack build', wasmPackBuild);
await step('npm install webui', npmInstallWebui);
await step('vite build', () => run('npx.cmd', ['vite', 'build'], { cwd: WEBUI }));

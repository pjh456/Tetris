import { wasmPackBuild, npmInstallWebui } from './lib/build.js';
import { WEBUI } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('wasm-pack build', wasmPackBuild);
await step('npm install webui', npmInstallWebui);
await step('vite dev server', () => run('npm', ['run', 'dev'], { cwd: WEBUI }));

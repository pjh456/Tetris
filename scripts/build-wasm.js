import { wasmPackBuild } from './lib/build.js';
import { step } from './lib/exec.js';

await step('wasm-pack', wasmPackBuild);

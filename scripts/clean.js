import { rm } from 'node:fs/promises';
import { ROOT, WASM_OUT } from './lib/env.js';
import { join } from 'node:path';
import { step } from './lib/exec.js';

await step('clean target/', () => rm(join(ROOT, 'target'), { recursive: true, force: true }));
await step(`clean ${WASM_OUT}/`, () => rm(WASM_OUT, { recursive: true, force: true }));
process.stdout.write('OK\n');

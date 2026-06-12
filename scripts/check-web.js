import { tscCheck } from './lib/verify.js';
import { step } from './lib/exec.js';

await step('tsc --noEmit', tscCheck);

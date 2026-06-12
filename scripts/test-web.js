import { vitestCheck } from './lib/verify.js';
import { step } from './lib/exec.js';

await step('vitest run', vitestCheck);

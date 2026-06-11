import { fullCheck } from './lib/verify.js';
import { step } from './lib/exec.js';

await step('check all', fullCheck);

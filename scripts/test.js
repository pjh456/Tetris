import { cargoTest, vitestCheck } from './lib/verify.js';
import { step } from './lib/exec.js';

await step('cargo test', cargoTest);
await step('vitest run', vitestCheck);

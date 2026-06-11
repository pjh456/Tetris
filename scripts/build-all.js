import { cargoBuildAll } from './lib/build.js';
import { step } from './lib/exec.js';

await step('cargo build --workspace --release', () => cargoBuildAll(true));

import { cargoBuild } from './lib/build.js';
import { CLI_CRATE } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('build tetris-cli --release', () => cargoBuild('tetris-cli', true));
await step('run', () => run('cargo', ['run', '--release'], { cwd: CLI_CRATE }));

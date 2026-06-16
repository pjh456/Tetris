import { WEBUI } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('vite preview', () =>
  run('npx.cmd', ['vite', 'preview', '--outDir', '../dist'], { cwd: WEBUI }),
);

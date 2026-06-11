import { WEBUI } from './lib/env.js';
import { run, step } from './lib/exec.js';

await step('vite preview', () =>
  run('npx', ['vite', 'preview', '--outDir', '../dist'], { cwd: WEBUI }),
);

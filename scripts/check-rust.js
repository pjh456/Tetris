import { runRoot } from './lib/exec.js';
import { step } from './lib/exec.js';

await step('cargo check', async () => {
    const r = await runRoot('cargo', ['check', '--workspace']);
    if (!r.ok) process.exit(r.code);
});

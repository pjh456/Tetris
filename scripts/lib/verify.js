import { runRoot } from './exec.js';
import { WEBUI } from './env.js';

export async function cargoTest() {
    const r = await runRoot('cargo', ['test', '--workspace']);
    if (!r.ok) process.exit(r.code);
}

export async function cargoClippy() {
    const r = await runRoot('cargo', ['clippy', '--workspace', '--', '-D', 'warnings']);
    if (!r.ok) process.exit(r.code);
}

export async function cargoFmtCheck() {
    const r = await runRoot('cargo', ['fmt', '--check']);
    if (!r.ok) process.exit(r.code);
}

export async function tscCheck() {
    const r = await runRoot('npx', ['tsc', '--noEmit'], { cwd: WEBUI });
    if (!r.ok) process.exit(r.code);
}

export async function eslintCheck() {
    const r = await runRoot('npx', ['eslint', 'src/'], { cwd: WEBUI });
    if (!r.ok) process.exit(r.code);
}

export async function vitestCheck() {
    const r = await runRoot('npx', ['vitest', 'run'], { cwd: WEBUI });
    if (!r.ok) process.exit(r.code);
}

export async function prettierCheck() {
    const r = await runRoot('npx', ['prettier', '--check', 'src/'], { cwd: WEBUI });
    if (!r.ok) process.exit(r.code);
}

export async function fullCheck() {
    await cargoTest();
    await cargoClippy();
    await cargoFmtCheck();
    await tscCheck();
    await eslintCheck();
    await vitestCheck();
    await prettierCheck();
    process.stdout.write('\nAll 7 checks passed.\n');
}

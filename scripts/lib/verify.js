import { runRoot } from './exec.js';

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

export async function fullCheck() {
    await cargoTest();
    await cargoClippy();
    await cargoFmtCheck();
    process.stdout.write('\nAll checks passed.\n');
}

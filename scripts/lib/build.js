import { writeFile, cp } from 'node:fs/promises';
import { join } from 'node:path';
import { ROOT, PKG, WASM_OUT, WASM_CRATE, WEBUI } from './env.js';
import { run } from './exec.js';

export async function cargoBuild(crate, release = true) {
    const args = ['build', '-p', crate];
    if (release) args.push('--release');
    const r = await run('cargo', args, { cwd: ROOT });
    if (!r.ok) process.exit(r.code);
}

export async function cargoBuildAll(release = true) {
    const args = ['build', '--workspace'];
    if (release) args.push('--release');
    const r = await run('cargo', args, { cwd: ROOT });
    if (!r.ok) process.exit(r.code);
}

export async function wasmPackBuild() {
    const r = await run('cargo', [
        'build', '-p', 'tetris-wasm',
        '--target', 'wasm32-unknown-unknown',
        '--release',
    ], { cwd: ROOT });
    if (!r.ok) process.exit(r.code);

    const r2 = await run('wasm-bindgen', [
        '--out-dir', PKG,
        '--out-name', 'tetris_wasm',
        '--target', 'web',
        '--no-typescript',
        join(ROOT, 'target', 'wasm32-unknown-unknown', 'release', 'tetris_wasm.wasm'),
    ], { cwd: ROOT });
    if (!r2.ok) process.exit(r2.code);

    await cp(PKG, WASM_OUT, { recursive: true, force: true });

    const envShim = `export function now() {
    return Date.now();
}
`;
    await writeFile(join(WASM_OUT, 'env.js'), envShim);
}

export async function npmInstallWebui() {
    const r = await run('npm.cmd', ['install'], { cwd: WEBUI });
    if (!r.ok) process.exit(r.code);
}

import { spawn } from 'node:child_process';
import { ROOT } from './env.js';

export function run(cmd, args = [], opts = {}) {
    return new Promise((resolve) => {
        const child = spawn(cmd, args, {
            cwd: ROOT,
            stdio: 'inherit',
            shell: true,
            ...opts,
        });

        child.on('close', (code) => {
            resolve({ ok: code === 0, code: code ?? 1 });
        });

        child.on('error', (err) => {
            process.stderr.write(`${cmd} failed: ${err.message}\n`);
            resolve({ ok: false, code: 1 });
        });
    });
}

export async function runCapture(cmd, args = [], opts = {}) {
    return new Promise((resolve) => {
        const child = spawn(cmd, args, {
            cwd: ROOT,
            stdio: ['pipe', 'pipe', 'pipe'],
            ...opts,
        });

        let stdout = '';
        let stderr = '';

        if (child.stdout) {
            child.stdout.on('data', (d) => { stdout += d.toString(); });
        }
        if (child.stderr) {
            child.stderr.on('data', (d) => { stderr += d.toString(); });
        }

        child.on('close', (code) => {
            resolve({ ok: code === 0, code: code ?? 1, stdout, stderr });
        });

        child.on('error', (err) => {
            resolve({ ok: false, code: 1, stdout, stderr: err.message });
        });
    });
}

export function runRoot(cmd, args, opts = {}) {
    return run(cmd, args, { cwd: ROOT, ...opts });
}

export async function execOrDie(cmd, args, opts = {}) {
    const r = await run(cmd, args, opts);
    if (!r.ok) process.exit(r.code);
    return r;
}

export async function step(label, fn) {
    process.stdout.write(`\n${'─'.repeat(50)}\n`);
    process.stdout.write(`  ${label}\n`);
    process.stdout.write(`${'─'.repeat(50)}\n`);
    const start = Date.now();
    try {
        await fn();
        const s = ((Date.now() - start) / 1000).toFixed(1);
        process.stdout.write(`\n✓ ${label} (${s}s)\n`);
    } catch (e) {
        process.stdout.write(`\n✗ ${label}\n`);
        throw e;
    }
}

import { spawn } from 'child_process';
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

function run(cmd, args, cwd) {
  return new Promise((resolve, reject) => {
    console.log(`\n> ${cmd} ${args.join(' ')}`);
    const child = spawn(cmd, args, {
      cwd,
      stdio: 'inherit',
      shell: true,
    });
    child.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`${cmd} exited with code ${code}`));
    });
  });
}

async function main() {
  try {
    console.log('=== Tetris Desktop Build ===\n');

    // Step 1: Build WASM
    console.log('[1/3] Building WASM...');
    await run('node', ['scripts/build-wasm.js'], root);

    // Step 2: Build webui
    console.log('[2/3] Building WebUI...');
    await run('npx', ['vite', 'build'], resolve(root, 'webui'));

    // Step 3: Build Tauri (release)
    console.log('[3/3] Building Tauri desktop app...');
    await run('npx', ['@tauri-apps/cli', 'build', '--config', 'crates/tetris-tauri/tauri.conf.json'], root);

    console.log('\n=== Build complete! ===');
  } catch (err) {
    console.error('Build failed:', err.message);
    process.exit(1);
  }
}

main();

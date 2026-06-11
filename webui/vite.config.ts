import { defineConfig } from 'vite';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  base: './',
  server: {
    fs: {
      allow: ['..'],
    },
  },
  resolve: {
    alias: {
      env: path.resolve(__dirname, 'wasm/env.js'),
    },
  },
  build: {
    outDir: path.resolve(__dirname, '..', 'dist'),
    emptyOutDir: true,
    assetsDir: 'assets',
  },
  optimizeDeps: {
    exclude: ['../wasm/tetris_wasm.js'],
  },
});

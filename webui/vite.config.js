import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
    server: {
        fs: {
            allow: ['..']
        }
    },
    resolve: {
        alias: {
            'env': path.resolve(__dirname, 'wasm/env.js')
        }
    }
});

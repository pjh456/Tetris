import { defineConfig } from 'vite';
import path from 'path';

export default defineConfig({
    server: {
        fs: {
            // 允许 Vite 开发服务器访问上一级目录（即工程根目录及其所有子目录）
            allow: ['..']
        }
    },
    resolve: {
        alias: {
            // 设置路径别名，指向编译产物文件夹
            // 注意：如果是原生的 ES Module 建议带上扩展名，但给文件夹配别名即可
            '@engine': path.resolve(__dirname, '../engine/build')
        }
    }
});
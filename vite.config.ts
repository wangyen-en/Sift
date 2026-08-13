import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import pkg from './package.json'

const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  base: './',
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    preserveSymlinks: true,
    alias: {
      '@': resolve(process.cwd(), 'src'),
    },
  },
  clearScreen: false,
  server: {
    fs: { strict: false },
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
}))

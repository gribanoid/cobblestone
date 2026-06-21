import { defineConfig } from 'vite'
import { resolve } from 'path'

const root = resolve(__dirname, '../..')

export default defineConfig({
  clearScreen: false,
  publicDir: resolve(root, 'packages/ui/public'),
  resolve: {
    alias: {
      '@cobblestone/api': resolve(root, 'packages/api/src'),
      '@cobblestone/ui': resolve(root, 'packages/ui/src'),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: 'localhost',
    watch: {
      ignored: ['**/crates/desktop/src-tauri/**'],
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})

import { defineConfig } from 'vite'
import { resolve } from 'path'

const root = resolve(__dirname, '../..')

export default defineConfig({
  publicDir: resolve(root, 'packages/ui/public'),
  resolve: {
    alias: {
      '@cobblestone/api': resolve(root, 'packages/api/src'),
      '@cobblestone/ui': resolve(root, 'packages/ui/src'),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': 'http://127.0.0.1:3000',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})

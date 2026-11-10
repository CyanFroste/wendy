import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

const dirname = import.meta.dirname
const host = process.env.TAURI_DEV_HOST

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  publicDir: path.resolve(dirname, 'assets'),
  build: {
    outDir: path.resolve(dirname, 'build'),
    chunkSizeWarningLimit: 2000,
    target: 'esnext',
  },
  resolve: {
    alias: { '@': path.resolve(dirname, 'src') },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    ...(host
      ? {
          host,
          hmr: { protocol: 'ws', host, port: 1421 },
        }
      : { host: false }),
    watch: {
      ignored: ['**/backend/**'],
    },
  },
}))

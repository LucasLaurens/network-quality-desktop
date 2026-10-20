import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

// Tauri expects a fixed dev port and serves the build from `dist`.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.spec.ts'],
  },
})

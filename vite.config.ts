import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'
import tauriConfig from './src-tauri/tauri.conf.json' with { type: 'json' }

// The dev port has one home, the Tauri config; Vite reads it from there.
const devPort = Number(new URL(tauriConfig.build.devUrl).port)

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: devPort, strictPort: true },
  test: {
    environment: 'happy-dom',
    include: ['tests/**/*.spec.ts'],
    // The module cache keys a file on its own content: a spec listing files by `import.meta.glob`
    // would keep the list it had, and the guards that read the sources would test stale code.
    fsModuleCache: false,
  },
})

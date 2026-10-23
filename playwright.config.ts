/// <reference types="node" />
import { defineConfig, devices } from '@playwright/test'
import tauriConfig from './src-tauri/tauri.conf.json' with { type: 'json' }

const isCi = process.env.CI !== undefined

// The preview pages, served by `pnpm dev:web` at the address the Tauri config owns.
export default defineConfig({
  testDir: 'tests/visual',
  testMatch: '*.pw.ts',
  // The popover is measured with the fonts of the systems that show it: on Linux the dot opens a
  // menu instead.
  ...(process.platform === 'linux' ? { grepInvert: /popover/ } : {}),
  forbidOnly: isCi,
  reporter: 'list',
  use: { ...devices['Desktop Chrome'], baseURL: tauriConfig.build.devUrl },
  projects: [
    { name: 'clair', use: { colorScheme: 'light' } },
    { name: 'sombre', use: { colorScheme: 'dark' } },
  ],
  webServer: { command: 'pnpm dev:web', url: tauriConfig.build.devUrl, reuseExistingServer: !isCi },
})

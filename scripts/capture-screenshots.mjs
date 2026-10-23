// Captures the README screenshots from the preview pages (`pnpm dev:web` must be running), in both
// themes, at the window sizes of tauri.conf.json. The previews use made-up data: no real network
// name or address ends up in the repository.
import { chromium } from '@playwright/test'
import { readFileSync } from 'node:fs'

/** @type {{ build: { devUrl: string }, app: { windows: { label: string, width: number, height: number }[] } }} */
const tauriConfig = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
/** @param {string} label */
const findWindow = (label) => {
  const found = tauriConfig.app.windows.find((window) => window.label === label)
  if (!found) throw new Error(`no ${label} window in tauri.conf.json`)
  return found
}
const MAIN = findWindow('main')
const POPOVER = findWindow('popover')
const BASE = tauriConfig.build.devUrl
const OUTPUT = 'docs/images'

const SHOTS = [
  { name: 'popover', size: POPOVER, path: '/tests/preview/popover.html?tests=on' },
  // Down to the evolution chart: the whole page would shrink everything on the README.
  { name: 'dashboard', size: { width: MAIN.width, height: 1060 }, path: '/tests/preview/dashboard.html' },
  { name: 'settings', size: MAIN, path: '/tests/preview/dashboard.html', page: 'settings', fullPage: true },
]

const browser = await chromium.launch()
for (const colorScheme of /** @type {const} */ (['light', 'dark'])) {
  for (const shot of SHOTS) {
    const page = await browser.newPage({
      viewport: { width: shot.size.width, height: shot.size.height },
      deviceScaleFactor: 2,
      baseURL: BASE,
      colorScheme,
    })
    await page.goto(shot.path)
    if (shot.page) await page.locator(`[data-page="${shot.page}"]`).click()
    await page.waitForLoadState('networkidle')
    await page.screenshot({ path: `${OUTPUT}/${shot.name}-${colorScheme}.png`, fullPage: shot.fullPage === true })
    await page.close()
  }
}
await browser.close()

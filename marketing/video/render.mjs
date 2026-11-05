// Renders stage.html frame by frame, then encodes the MP4. Needs `pnpm dev:web` running.
// `node marketing/video/render.mjs 3 12 25` only writes stills at those seconds;
// add `square` for the 1080x1080 cut.
import { chromium } from '@playwright/test'
import { execFileSync } from 'node:child_process'
import { mkdirSync, rmSync } from 'node:fs'
import process from 'node:process'

const FPS = 30
const DURATION = 44.5
const OUTPUT = 'marketing/video'
const FRAMES = `${OUTPUT}/.frames`
const square = process.argv.includes('square')
const stills = process.argv
  .slice(2)
  .filter((arg) => arg !== 'square')
  .map(Number)
const size = square ? { width: 1080, height: 1080 } : { width: 1920, height: 1080 }
const name = square ? 'qualite-de-connexion-carre' : 'qualite-de-connexion'

const browser = await chromium.launch()
const page = await browser.newPage({ viewport: size, colorScheme: 'dark' })
await page.goto(`http://localhost:1420/marketing/video/stage.html${square ? '?format=square' : ''}`)
await page.waitForLoadState('networkidle')
for (const frame of page.frames().slice(1)) await frame.waitForSelector('h1, h2')
await page.waitForTimeout(1000)

if (stills.length > 0) {
  for (const second of stills) {
    await page.evaluate((t) => globalThis.seek(t), second)
    await page.screenshot({ path: `${OUTPUT}/still-${name}-${second}.png` })
  }
} else {
  rmSync(FRAMES, { recursive: true, force: true })
  mkdirSync(FRAMES)
  const count = Math.round(DURATION * FPS)
  for (let index = 0; index < count; index++) {
    await page.evaluate((t) => globalThis.seek(t), index / FPS)
    await page.screenshot({ path: `${FRAMES}/${String(index).padStart(5, '0')}.jpg`, type: 'jpeg', quality: 95 })
  }
  execFileSync('ffmpeg', [
    '-y',
    '-loglevel',
    'error',
    '-framerate',
    String(FPS),
    '-i',
    `${FRAMES}/%05d.jpg`,
    '-c:v',
    'libx264',
    '-preset',
    'slow',
    '-crf',
    '18',
    '-pix_fmt',
    'yuv420p',
    '-movflags',
    '+faststart',
    `${OUTPUT}/${name}.mp4`,
  ])
  rmSync(FRAMES, { recursive: true, force: true })
}
await browser.close()

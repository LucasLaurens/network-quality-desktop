// Builds the desktop bundle and, on macOS, copies the .app into /Applications.
import { execFileSync } from 'node:child_process'
import { cpSync, existsSync, readdirSync, rmSync } from 'node:fs'
import { join } from 'node:path'

const bundleDir = join('target', 'release', 'bundle')

execFileSync('pnpm', ['tauri', 'build'], { stdio: 'inherit', shell: process.platform === 'win32' })

if (process.platform !== 'darwin') {
  console.log(`Installeurs générés dans ${bundleDir} : lancer celui de votre système.`)
  process.exit(0)
}

const macosDir = join(bundleDir, 'macos')
const app = readdirSync(macosDir).find((name) => name.endsWith('.app'))
if (!app) throw new Error(`Aucune application trouvée dans ${macosDir}`)

const target = join('/Applications', app)
if (existsSync(target)) rmSync(target, { recursive: true })
cpSync(join(macosDir, app), target, { recursive: true })
console.log(`Installée dans ${target}`)

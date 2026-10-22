import { readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { findMisnamedFunctions, readAllowedVerbs } from './method-names.mjs'

const FOLDERS = ['src', 'scripts', 'tests/fakes', 'tests/preview']
// Written by tauri-specta: its names follow the Rust commands, checked on the Rust side.
const GENERATED = new Set([join('src', 'bindings.ts')])
const SOURCE = /\.(ts|mjs)$/

const verbs = readAllowedVerbs(readFileSync('scripts/method-verbs.txt', 'utf8'))
const misnamed = FOLDERS.flatMap((folder) =>
  readdirSync(folder, { recursive: true, encoding: 'utf8' })
    .map((file) => join(folder, file))
    .filter((file) => SOURCE.test(file) && !GENERATED.has(file))
    .flatMap((file) =>
      findMisnamedFunctions(readFileSync(file, 'utf8'), file, verbs).map((name) => `${file}: ${name}`),
    ),
)
if (misnamed.length > 0) {
  console.error(`Noms à commencer par un verbe (scripts/method-verbs.txt) :\n${misnamed.join('\n')}`)
  process.exit(1)
}

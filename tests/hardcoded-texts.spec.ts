import { describe, expect, it } from 'vitest'
import { findHardcodedTexts } from '../scripts/hardcoded-texts.mjs'
import interfaceTexts from './fakes/interface-texts.json'

const SOURCES = import.meta.glob<string>(['../src/**/*.{vue,ts}', '!../src/bindings.ts'], {
  query: '?raw',
  import: 'default',
  eager: true,
})
const RUST_SOURCES = import.meta.glob<string>('../src-tauri/src/**/*.rs', {
  query: '?raw',
  import: 'default',
  eager: true,
})
const KEY = /['"`](ui-[a-z0-9-]+)['"`]/g

function listKeys(source: string): string[] {
  return [...source.matchAll(KEY)].flatMap((match) => (match[1] === undefined ? [] : [match[1]]))
}

function find(template: string, script = ''): string[] {
  return findHardcodedTexts(`<script setup lang="ts">${script}</script><template>${template}</template>`, 'Sample.vue')
}

describe('findHardcodedTexts', () => {
  it('finds a sentence written in a template', () => {
    // Act
    const found = find('<p>Tests de débit désactivés</p><p>{{ say("ui-cancel") }}</p>')

    // Assert
    expect(found).toEqual(['Tests de débit désactivés'])
  })

  it('finds a text attribute but leaves the other attributes alone', () => {
    // Act
    const found = find('<nav aria-label="Pages" class="pages" data-page="dashboard"></nav>')

    // Assert
    expect(found).toEqual(['Pages'])
  })

  it('finds a text in a template expression', () => {
    // Act
    const found = find(
      `<button :aria-current="isHere ? 'page' : undefined">{{ busy ? 'Exporter…' : 'Annuler' }}</button>`,
    )

    // Assert
    expect(found).toEqual(['Exporter…', 'Annuler'])
  })

  it('finds a label in the script and leaves keys, errors and logs alone', () => {
    // Arrange
    const script = `
      import { inject } from 'vue'
      const PAGES = [{ value: 'dashboard', label: 'Réglages' }, { value: 'settings', labelKey: 'ui-settings' }]
      const OPTIONS = ['Il y a 1 h', 'fr-FR', \`var(--\${state})\`]
      if (!client) throw new Error('NetworkQualityClient is not provided')
      console.error('Reading the settings failed', error)
      const isEscape = event.key === 'Escape'
    `

    // Act
    const found = find('<p></p>', script)

    // Assert
    expect(found).toEqual(['Réglages', 'Il y a 1 h'])
  })

  it('reads a plain TypeScript module too', () => {
    // Act
    const found = findHardcodedTexts("export const PROFILES = [{ value: 'quick', label: 'Test rapide' }]", 'sample.ts')

    // Assert
    expect(found).toEqual(['Test rapide'])
  })
})

describe('the windows', () => {
  it('take every word from the catalogue', () => {
    // Act
    const found = Object.entries(SOURCES).flatMap(([file, source]) =>
      findHardcodedTexts(source, file).map((text) => `${file}: ${text}`),
    )

    // Assert
    expect(found).toEqual([])
  })

  it('ask only for texts the catalogue has', () => {
    // Act
    const missing = Object.values(SOURCES)
      .flatMap(listKeys)
      .filter((key) => !(key in interfaceTexts))

    // Assert
    expect(missing).toEqual([])
  })

  it('leave no interface text unused', () => {
    // Arrange
    const used = new Set([...Object.values(SOURCES), ...Object.values(RUST_SOURCES)].flatMap(listKeys))

    // Act
    const unused = Object.keys(interfaceTexts).filter((key) => !used.has(key))

    // Assert
    expect(unused).toEqual([])
  })
})

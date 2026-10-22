import { faker } from '@faker-js/faker'
import { describe, expect, it } from 'vitest'
import { isDocumentationMissing } from '../scripts/docs-check.mjs'

const NO_DOC_LABEL = 'sans-doc'

describe('isDocumentationMissing', () => {
  it('fails a change to the code that leaves the docs alone', () => {
    // Act & Assert
    expect(isDocumentationMissing([`crates/domain/src/${faker.word.noun()}.rs`], [])).toBe(true)
  })

  it('accepts code that comes with a doc or the README', () => {
    // Act & Assert
    expect(isDocumentationMissing(['src/App.vue', `docs/technique/${faker.word.noun()}.md`], [])).toBe(false)
    expect(isDocumentationMissing(['src-tauri/src/lib.rs', 'README.md'], [])).toBe(false)
  })

  it('accepts code without docs once the label says so', () => {
    // Act & Assert
    expect(isDocumentationMissing(['scripts/install-app.mjs'], [faker.word.noun(), NO_DOC_LABEL])).toBe(false)
  })

  it('asks nothing of a change that touches no code', () => {
    // Act & Assert
    expect(isDocumentationMissing(['.github/workflows/ci.yml', 'package.json'], [])).toBe(false)
  })
})

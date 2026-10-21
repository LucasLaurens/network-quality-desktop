import { faker } from '@faker-js/faker'
import { describe, expect, it } from 'vitest'
import { isConventionalCommitMessage } from '../scripts/commit-message.mjs'

describe('isConventionalCommitMessage', () => {
  it('accepts a typed, scoped header', () => {
    // Act & Assert
    expect(isConventionalCommitMessage(`feat(domain): ${faker.hacker.phrase()}`)).toBe(true)
  })

  it('accepts a breaking change marker and a body', () => {
    // Act & Assert
    expect(isConventionalCommitMessage(`refactor!: ${faker.lorem.words(3)}\n\n${faker.lorem.sentence()}`)).toBe(true)
  })

  it('rejects a header without a type', () => {
    // Act & Assert
    expect(isConventionalCommitMessage(faker.lorem.sentence())).toBe(false)
  })

  it('rejects an unknown type', () => {
    // Act & Assert
    expect(isConventionalCommitMessage(`update: ${faker.lorem.words(3)}`)).toBe(false)
  })
})

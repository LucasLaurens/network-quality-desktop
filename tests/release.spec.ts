import { faker } from '@faker-js/faker'
import { describe, expect, it } from 'vitest'
import { composeMergeArguments, selectReleasePullRequest } from '../scripts/release.mjs'

function createPullRequest() {
  return { number: faker.number.int({ min: 1, max: 9999 }), title: `chore(main): release ${faker.system.semver()}` }
}

describe('release', () => {
  it('takes the release pull request release-please keeps open, if any', () => {
    // Arrange
    const pending = createPullRequest()

    // Act & Assert
    expect(selectReleasePullRequest([pending])).toEqual(pending)
    expect(selectReleasePullRequest([])).toBeUndefined()
  })

  it('refuses to choose between two release pull requests', () => {
    // Act & Assert
    expect(() => selectReleasePullRequest([createPullRequest(), createPullRequest()])).toThrow()
  })

  it('squash-merges it under its own title, with nothing added to the message', () => {
    // Arrange
    const pending = createPullRequest()

    // Act
    const command = composeMergeArguments(pending)

    // Assert
    expect(command).toEqual([
      'pr',
      'merge',
      String(pending.number),
      '--squash',
      '--subject',
      pending.title,
      '--body',
      '',
    ])
  })
})

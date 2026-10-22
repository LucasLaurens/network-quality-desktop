// The release pull request release-please keeps open, and how `make release` merges it.

/** @typedef {{ number: number, title: string }} PullRequest */

/** The label release-please puts on the release pull request until it is merged. */
export const RELEASE_LABEL = 'autorelease: pending'

/**
 * `undefined` when no release is waiting. O(1).
 * @param {readonly PullRequest[]} pending the open pull requests carrying `RELEASE_LABEL`
 * @returns {PullRequest | undefined}
 */
export function selectReleasePullRequest(pending) {
  if (pending.length > 1) {
    throw new Error(`${String(pending.length)} PR de release ouvertes : en fermer une à la main.`)
  }
  return pending[0]
}

/**
 * Squash, under the pull request's title: the message stays exactly what release-please wrote,
 * with no trailer added by GitHub. O(1).
 * @param {PullRequest} pullRequest
 * @returns {string[]} the arguments of `gh`
 */
export function composeMergeArguments(pullRequest) {
  return ['pr', 'merge', String(pullRequest.number), '--squash', '--subject', pullRequest.title, '--body', '']
}

// Conventional Commits, in English: type(optional-scope)!: subject
const CONVENTIONAL_HEADER = /^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(\([a-z0-9-]+\))?!?: \S.*$/

/** @param {string} message */
export function isConventionalCommitMessage(message) {
  const header = message.split('\n', 1)[0] ?? ''
  return header.startsWith('Merge ') || CONVENTIONAL_HEADER.test(header)
}

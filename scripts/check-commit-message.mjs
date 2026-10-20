import { readFileSync } from 'node:fs'
import { isConventionalCommitMessage } from './commit-message.mjs'

const file = process.argv[2]
if (!file) throw new Error('Usage: check-commit-message.mjs <message-file>')

const message = readFileSync(file, 'utf8')
if (!isConventionalCommitMessage(message)) {
  console.error('commit-msg: expected "type(scope): subject" (Conventional Commits, in English).')
  process.exit(1)
}

// Run by the « Documentation » workflow on a pull request: the event file gives its base, its
// head and its labels.
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { isDocumentationMissing } from './docs-check.mjs'

const eventPath = process.env['GITHUB_EVENT_PATH']
if (!eventPath) throw new Error('GITHUB_EVENT_PATH is not set: run this on a pull_request event')

/** @type {{ pull_request: { base: { sha: string }, head: { sha: string }, labels: { name: string }[] } }} */
const event = JSON.parse(readFileSync(eventPath, 'utf8'))
const { base, head, labels } = event.pull_request
const changedFiles = execFileSync('git', ['diff', '--name-only', `${base.sha}...${head.sha}`], { encoding: 'utf8' })
  .split('\n')
  .filter((file) => file !== '')
if (
  isDocumentationMissing(
    changedFiles,
    labels.map((label) => label.name),
  )
) {
  console.error('Le code change sans docs/ ni README.md : documente le changement, ou pose le label « sans-doc ».')
  process.exit(1)
}

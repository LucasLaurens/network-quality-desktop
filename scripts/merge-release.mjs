// `make release`: merges the waiting release pull request; the Release workflow then tags the
// version, publishes the release and builds the installers.
import { execFileSync } from 'node:child_process'
import { RELEASE_LABEL, composeMergeArguments, selectReleasePullRequest } from './release.mjs'

const listed = execFileSync(
  'gh',
  ['pr', 'list', '--state', 'open', '--label', RELEASE_LABEL, '--json', 'number,title'],
  { encoding: 'utf8' },
)
const pullRequest = selectReleasePullRequest(/** @type {import('./release.mjs').PullRequest[]} */ (JSON.parse(listed)))
if (!pullRequest) {
  console.log('Aucune release en attente : rien de nouveau depuis la dernière.')
  process.exit(0)
}
console.log(`Fusion de « ${pullRequest.title} » (#${String(pullRequest.number)})`)
execFileSync('gh', composeMergeArguments(pullRequest), { stdio: 'inherit' })

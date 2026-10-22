// `make release`: merges the waiting release pull request; the Release workflow then tags the
// version, publishes the release and builds the installers.
import { execFileSync } from 'node:child_process'
import { RELEASE_LABEL, composeMergeArguments, isLockfileSynced, selectReleasePullRequest } from './release.mjs'

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
const viewed = execFileSync('gh', ['pr', 'view', String(pullRequest.number), '--json', 'files'], { encoding: 'utf8' })
const { files } = /** @type {{ files: { path: string }[] }} */ (JSON.parse(viewed))
if (!isLockfileSynced(files.map((file) => file.path))) {
  console.error("Cargo.lock n'est pas encore à jour sur la PR de release : réessayer dans une minute.")
  process.exit(1)
}
console.log(`Fusion de « ${pullRequest.title} » (#${String(pullRequest.number)})`)
execFileSync('gh', composeMergeArguments(pullRequest), { stdio: 'inherit' })

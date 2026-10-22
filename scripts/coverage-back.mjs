// Coverage gates, without a shell: the same arguments on the three OSes.
import { execFileSync } from 'node:child_process'

// Test scaffolding (fakes, the shared repository contract) would inflate the figure.
const SCAFFOLDING = String.raw`(test_support|repository_contract)\.rs`

execFileSync('cargo', ['llvm-cov', '-p', 'domain', '--summary-only', '--fail-under-lines', '90'], {
  stdio: 'inherit',
})
execFileSync(
  'cargo',
  [
    'llvm-cov',
    '-p',
    'application',
    '--all-features',
    '--summary-only',
    '--ignore-filename-regex',
    SCAFFOLDING,
    '--fail-under-lines',
    '80',
  ],
  { stdio: 'inherit' },
)

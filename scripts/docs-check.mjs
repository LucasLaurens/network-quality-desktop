// A pull request that changes the code says what changed in docs/ or README.md, unless it carries
// the « sans-doc » label (a pure refactor, a typo).
const CODE_FOLDERS = ['crates/', 'src/', 'src-tauri/', 'scripts/', 'tests/']
const DOC_FOLDER = 'docs/'
const README = 'README.md'
const NO_DOC_LABEL = 'sans-doc'

/**
 * O(f + l) for f changed files and l labels.
 * @param {readonly string[]} changedFiles paths from the repository root
 * @param {readonly string[]} labels
 */
export function isDocumentationMissing(changedFiles, labels) {
  const touchesCode = changedFiles.some((file) => CODE_FOLDERS.some((folder) => file.startsWith(folder)))
  const touchesDocs = changedFiles.some((file) => file.startsWith(DOC_FOLDER) || file === README)
  return touchesCode && !touchesDocs && !labels.includes(NO_DOC_LABEL)
}

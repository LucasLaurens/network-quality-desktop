// A function is an action: its name starts with a verb of scripts/method-verbs.txt, the list the
// Rust check reads too. Only what a module exports is checked: exported functions, the methods of
// exported classes and interfaces. Exempt: constructors, accessors, the from/to/is/has idioms.
import ts from 'typescript'

const EXEMPT_PREFIXES = new Set(['from', 'to', 'is', 'has'])

/**
 * One verb a line; blank lines and `#` comments are skipped. O(v) for v verbs.
 * @param {string} text the content of scripts/method-verbs.txt
 * @returns {Set<string>}
 */
export function readAllowedVerbs(text) {
  return new Set(
    text
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line !== '' && !line.startsWith('#')),
  )
}

/**
 * The exported names in `source` that do not start with an allowed verb. O(n) for n top-level
 * statements and their members.
 * @param {string} source
 * @param {string} fileName
 * @param {Set<string>} verbs
 * @returns {string[]}
 */
export function findMisnamedFunctions(source, fileName, verbs) {
  const file = ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, true)
  return file.statements
    .filter(isExported)
    .flatMap(listFunctionNames)
    .filter((name) => !isWellNamed(name, verbs))
}

/** @param {ts.Statement} statement */
function isExported(statement) {
  return (
    ts.canHaveModifiers(statement) &&
    (ts.getModifiers(statement) ?? []).some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword)
  )
}

/**
 * @param {ts.Statement} statement
 * @returns {string[]}
 */
function listFunctionNames(statement) {
  if (ts.isFunctionDeclaration(statement)) return statement.name ? [statement.name.text] : []
  if (ts.isVariableStatement(statement)) {
    return statement.declarationList.declarations
      .filter((declaration) => declaration.initializer && ts.isFunctionLike(declaration.initializer))
      .flatMap((declaration) => (ts.isIdentifier(declaration.name) ? [declaration.name.text] : []))
  }
  if (ts.isClassDeclaration(statement) || ts.isInterfaceDeclaration(statement)) {
    return statement.members
      .filter((member) => ts.isMethodDeclaration(member) || ts.isMethodSignature(member))
      .flatMap((member) => (member.name && ts.isIdentifier(member.name) ? [member.name.text] : []))
  }
  return []
}

/**
 * The first word of a camelCase name. O(length).
 * @param {string} name
 * @param {Set<string>} verbs
 */
function isWellNamed(name, verbs) {
  const firstWord = /^[a-z]+/.exec(name)?.[0] ?? ''
  return EXEMPT_PREFIXES.has(firstWord) || verbs.has(firstWord)
}

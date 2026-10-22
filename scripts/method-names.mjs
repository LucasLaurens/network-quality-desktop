// A function is an action: its name starts with a verb of scripts/method-verbs.txt, the list the
// Rust check reads too. Only what a module exports is checked: exported functions, the methods of
// exported classes and interfaces, with the from/to/is/has idioms. Exempt: constructors, accessors.
import ts from 'typescript'

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
  const exportedByName = new Set(file.statements.flatMap(listNamesExportedApart))
  return file.statements
    .filter(
      (statement) => isExported(statement) || listDeclaredNames(statement).some((name) => exportedByName.has(name)),
    )
    .flatMap(listFunctionNames)
    .filter((name) => !isWellNamed(name, verbs))
}

/**
 * The local names of `export { a, b }`; nothing for a re-export from another module. O(e).
 * @param {ts.Statement} statement
 * @returns {string[]}
 */
function listNamesExportedApart(statement) {
  if (!ts.isExportDeclaration(statement) || statement.moduleSpecifier || !statement.exportClause) return []
  if (!ts.isNamedExports(statement.exportClause)) return []
  return statement.exportClause.elements.map((element) => (element.propertyName ?? element.name).text)
}

/**
 * The names a statement declares. O(d) for d declarations.
 * @param {ts.Statement} statement
 * @returns {string[]}
 */
function listDeclaredNames(statement) {
  if (ts.isVariableStatement(statement)) {
    return statement.declarationList.declarations.flatMap((declaration) =>
      ts.isIdentifier(declaration.name) ? [declaration.name.text] : [],
    )
  }
  if ((ts.isFunctionDeclaration(statement) || ts.isClassDeclaration(statement)) && statement.name) {
    return [statement.name.text]
  }
  return ts.isInterfaceDeclaration(statement) ? [statement.name.text] : []
}

/**
 * O(m) for m modifiers.
 * @param {ts.Statement} statement
 */
function isExported(statement) {
  return (
    ts.canHaveModifiers(statement) &&
    (ts.getModifiers(statement) ?? []).some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword)
  )
}

/**
 * The functions a statement declares: a function, function-valued constants, the methods and
 * function-typed properties of a class or an interface. O(m) for m members.
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
      .filter(isFunctionMember)
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
  const firstWord = /^_*([a-z]+)/.exec(name)?.[1] ?? ''
  return verbs.has(firstWord)
}

/**
 * A method, or a property whose type or value is a function. O(1).
 * @param {ts.ClassElement | ts.TypeElement} member
 */
function isFunctionMember(member) {
  return (
    ts.isMethodDeclaration(member) ||
    ts.isMethodSignature(member) ||
    (ts.isPropertySignature(member) && member.type !== undefined && ts.isFunctionTypeNode(member.type)) ||
    (ts.isPropertyDeclaration(member) && member.initializer !== undefined && ts.isFunctionLike(member.initializer))
  )
}

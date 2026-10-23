// Every word of the windows comes from the Fluent catalogue (`read_interface_texts`): this finds
// the ones written in a component instead. In a template, any text and any text attribute count;
// so does any word written in `{{ }}` or bound to a text attribute, a catalogue key aside. In the
// other bindings (`:class`, `:aria-current`, `@click`…) and in a script, a string counts only when
// it reads as words: a space, a letter beyond ASCII, an ellipsis, or, in a script, the value of a
// `label`-like property. So a lone ASCII word in a script, `'inconnue'` as well as `'Escape'`, is
// left to the review: nothing tells it from a key name.
import ts from 'typescript'
import { parse } from 'vue/compiler-sfc'

const TEXT_ATTRIBUTES = new Set([
  'alt',
  'aria-description',
  'aria-label',
  'aria-placeholder',
  'aria-roledescription',
  'aria-valuetext',
  'label',
  'placeholder',
  'title',
])
const LABEL_PROPERTIES = new Set(['hint', 'label', 'message', 'text', 'title', 'warning'])
const VUE_NODE = { element: 1, text: 2, interpolation: 5, attribute: 6, directive: 7 }
const LETTER = /\p{L}/u
const BEYOND_ASCII = /[^\p{ASCII}]/u
const WORDS = /\s|[^\p{ASCII}]/u
const CATALOGUE_KEY = /^ui-[a-z0-9-]+$/
/** Where a string is read: how sure the check can be that it reaches the window. */
const READING = { script: 'script', shown: 'shown', bound: 'bound' }

/**
 * The few fields of the Vue template syntax tree this check reads.
 * @typedef {{ content?: string }} VueExpression
 * @typedef {{ type: number, name: string, value?: { content: string }, arg?: VueExpression, exp?: VueExpression }} VueProp
 * @typedef {{ type: number, content?: string | VueExpression, props?: VueProp[], children?: VueNode[] }} VueNode
 */

/**
 * The texts written in a component or a module rather than read from the catalogue, in the order
 * they appear (script first). O(n) for a source of n characters.
 * @param {string} source
 * @param {string} fileName
 * @returns {string[]}
 */
export function findHardcodedTexts(source, fileName) {
  if (!fileName.endsWith('.vue')) return findScriptTexts(source, fileName, READING.script)
  const { descriptor } = parse(source, { filename: fileName })
  const scripts = [descriptor.script, descriptor.scriptSetup].flatMap((block) => (block ? [block.content] : []))
  return [
    ...scripts.flatMap((script) => findScriptTexts(script, `${fileName}.ts`, READING.script)),
    ...(descriptor.template?.ast ? findTemplateTexts(/** @type {VueNode} */ (descriptor.template.ast), fileName) : []),
  ]
}

/**
 * O(n) for n template nodes and their expressions.
 * @param {VueNode} node
 * @param {string} fileName
 * @returns {string[]}
 */
function findTemplateTexts(node, fileName) {
  /** @type {string[]} */
  const found = []
  const content = node.content
  if (node.type === VUE_NODE.text && typeof content === 'string' && LETTER.test(content)) {
    found.push(content.replace(/\s+/g, ' ').trim())
  }
  if (node.type === VUE_NODE.interpolation && typeof content === 'object') {
    found.push(...findExpressionTexts(content, fileName, READING.shown))
  }
  for (const prop of node.props ?? []) {
    const value = prop.value?.content ?? ''
    if (prop.type === VUE_NODE.attribute && TEXT_ATTRIBUTES.has(prop.name) && LETTER.test(value)) found.push(value)
    if (prop.type === VUE_NODE.directive) {
      const isTextBinding = prop.name === 'bind' && TEXT_ATTRIBUTES.has(prop.arg?.content ?? '')
      found.push(...findExpressionTexts(prop.exp, fileName, isTextBinding ? READING.shown : READING.bound))
    }
  }
  for (const child of node.children ?? []) found.push(...findTemplateTexts(child, fileName))
  return found
}

/**
 * @param {VueExpression | undefined} expression
 * @param {string} fileName
 * @param {string} reading one of `READING`
 * @returns {string[]}
 */
function findExpressionTexts(expression, fileName, reading) {
  if (!expression?.content) return []
  return findScriptTexts(`(${expression.content})`, `${fileName}.ts`, reading)
}

/**
 * O(n) for n syntax nodes.
 * @param {string} code
 * @param {string} fileName
 * @param {string} reading one of `READING`
 * @returns {string[]}
 */
function findScriptTexts(code, fileName, reading) {
  const file = ts.createSourceFile(fileName, code, ts.ScriptTarget.Latest, true)
  /** @type {string[]} */
  const found = []
  /** @param {ts.Node} node */
  const visit = (node) => {
    if (isOutsideTheWindow(node)) return
    const text = readLiteralText(node)
    if (text !== null && isText(text, node, reading)) found.push(text)
    ts.forEachChild(node, visit)
  }
  visit(file)
  return found
}

/**
 * Module paths, types, logs, errors and property names never reach the window.
 * @param {ts.Node} node
 * @returns {boolean}
 */
function isOutsideTheWindow(node) {
  if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node) || ts.isLiteralTypeNode(node)) return true
  if (ts.isNewExpression(node)) return ts.isIdentifier(node.expression) && node.expression.text.endsWith('Error')
  if (ts.isCallExpression(node)) {
    const callee = node.expression
    return (
      ts.isPropertyAccessExpression(callee) &&
      ts.isIdentifier(callee.expression) &&
      callee.expression.text === 'console'
    )
  }
  return node.parent !== undefined && ts.isPropertyAssignment(node.parent) && node.parent.name === node
}

/**
 * The written part of a string or a template string; `null` for any other node.
 * @param {ts.Node} node
 * @returns {string | null}
 */
function readLiteralText(node) {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text
  if (ts.isTemplateExpression(node))
    return [node.head.text, ...node.templateSpans.map((span) => span.literal.text)].join('')
  return null
}

/**
 * @param {string} text
 * @param {ts.Node} node
 * @param {string} reading one of `READING`
 * @returns {boolean}
 */
function isText(text, node, reading) {
  if (!LETTER.test(text) || CATALOGUE_KEY.test(text)) return false
  if (reading === READING.shown) return true
  if (reading === READING.bound) return BEYOND_ASCII.test(text)
  if (WORDS.test(text.trim())) return true
  const parent = node.parent
  return ts.isPropertyAssignment(parent) && ts.isIdentifier(parent.name) && LABEL_PROPERTIES.has(parent.name.text)
}

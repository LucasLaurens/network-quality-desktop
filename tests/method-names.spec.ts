import { describe, expect, it } from 'vitest'
import { findMisnamedFunctions, readAllowedVerbs } from '../scripts/method-names.mjs'
import verbsText from '../scripts/method-verbs.txt?raw'

const verbs = readAllowedVerbs(verbsText)

function find(source: string): string[] {
  return findMisnamedFunctions(source, 'sample.ts', verbs)
}

describe('findMisnamedFunctions', () => {
  it('rejects an exported function named like a thing', () => {
    // Act
    const misnamed = find('export function totalPrice() {} export function computeTotalPrice() {}')

    // Assert
    expect(misnamed).toEqual(['totalPrice'])
  })

  it('checks exported arrow functions, class methods and interface methods', () => {
    // Arrange
    const source = `
      export const shippingCity = () => ''
      export const readCity = () => ''
      export class Order { candidates(): void {} readOrder(): void {} get total(): number { return 0 } }
      export interface Repository { purgeList(): void; rows(): string[] }
    `

    // Act
    const misnamed = find(source)

    // Assert
    expect(misnamed).toEqual(['shippingCity', 'candidates', 'rows'])
  })

  it('follows a function exported by name, and the function-typed members of an interface', () => {
    // Arrange
    const source = `
      function priceList() {}
      function readPrices() {}
      export { priceList, readPrices }
      export interface Port { readValue: () => string; valueLabel: () => string; label: string }
    `

    // Act
    const misnamed = find(source)

    // Assert
    expect(misnamed).toEqual(['priceList', 'valueLabel'])
  })

  it('reads arrow functions held by a class, and skips the underscore of a private name', () => {
    // Arrange
    const source =
      'export class Store { totalSize = () => 0; readSize = () => 0; _readCache(): void {} _cache(): void {} }'

    // Act
    const misnamed = find(source)

    // Assert
    expect(misnamed).toEqual(['totalSize', '_cache'])
  })

  it('leaves what is not exported, constructors, conversions and predicates alone', () => {
    // Arrange
    const source = `
      function helperThing() {}
      export class Meter { constructor() {} fromOctets() {} toText() {} isUp() {} hasRoute() {} }
    `

    // Act & Assert
    expect(find(source)).toEqual([])
  })
})

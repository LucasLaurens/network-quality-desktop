import { faker } from '@faker-js/faker'
import { describe, expect, it } from 'vitest'
import { formatInterfaceText } from '../src/composables/interface-texts'

describe('formatInterfaceText', () => {
  it('fills every slot, a repeated one included', () => {
    // Arrange
    const name = faker.person.firstName()
    const texts = { 'ui-greeting': 'Bonjour {name}, au revoir {name}' }

    // Act
    const text = formatInterfaceText(texts, 'ui-greeting', { name })

    // Assert
    expect(text).toBe(`Bonjour ${name}, au revoir ${name}`)
  })

  it('leaves a value that looks like a slot as it is', () => {
    // Arrange
    const texts = { 'ui-note': 'Note : {value}' }

    // Act
    const text = formatInterfaceText(texts, 'ui-note', { value: '{other}' })

    // Assert
    expect(text).toBe('Note : {other}')
  })

  it('refuses a key the catalogue lacks and a slot left empty', () => {
    // Arrange
    const texts = { 'ui-note': 'Note : {value}' }

    // Act & Assert
    expect(() => formatInterfaceText(texts, `ui-${faker.word.noun()}`)).toThrow('missing from the catalogue')
    expect(() => formatInterfaceText(texts, 'ui-note')).toThrow('needs the slot value')
  })
})

import { inject, type InjectionKey } from 'vue'
import type { InterfaceTexts } from '../services/network-quality-client'
/** What fills the `{name}` slots of a text. */
export type TextSlots = Record<string, string>
export type SayText = (key: string, slots?: TextSlots) => string

export const INTERFACE_TEXTS_KEY: InjectionKey<InterfaceTexts> = Symbol('interface-texts')

const SLOT = /\{([\w-]+)\}/g

/**
 * A catalogue text with its slots filled. O(length of the text). Throws on a key or a slot the
 * catalogue and the caller disagree on: a programming error the tests catch.
 */
export function formatInterfaceText(texts: InterfaceTexts, key: string, slots: TextSlots = {}): string {
  const text = texts[key]
  if (text === undefined) throw new Error(`${key} is missing from the catalogue`)
  return text.replace(SLOT, (_, name: string) => {
    const value = slots[name]
    if (value === undefined) throw new Error(`${key} needs the slot ${name}`)
    return value
  })
}

/** The function a component says its texts with. */
export function useInterfaceTexts(): SayText {
  const texts = inject(INTERFACE_TEXTS_KEY)
  if (!texts) throw new Error('InterfaceTexts are not provided')
  return (key, slots) => formatInterfaceText(texts, key, slots)
}

import { faker } from '@faker-js/faker'
import { flushPromises, mount, type DOMWrapper } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SettingsView from '../src/components/SettingsView.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import { InMemoryNetworkQualityClient } from './fakes/in-memory-network-quality-client'

async function mountSettings(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(SettingsView, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
  await flushPromises()
  return wrapper
}

const createAddress = (): string => faker.internet.ipv4()

/** Typing fires `input` only; `change` comes on Enter or when the field loses focus. */
async function type(field: Omit<DOMWrapper<Element>, 'exists'>, text: string): Promise<void> {
  ;(field.element as HTMLInputElement).value = text
  await field.trigger('input')
}

describe('SettingsView', () => {
  it('shows the settings as last saved', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()

    // Act
    const wrapper = await mountSettings(client)

    // Assert
    expect((wrapper.get('#setting-launch').element as HTMLInputElement).checked).toBe(true)
    expect((wrapper.get('[value="system"]').element as HTMLInputElement).checked).toBe(true)
    expect(wrapper.find('#setting-server').exists()).toBe(false)
  })

  it('saves a switch at once and says when', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)

    // Act
    await wrapper.get('#setting-launch').setValue(false)
    await flushPromises()

    // Assert
    expect(client.savedForms).toEqual([{ launchAtLogin: false, appearance: 'system', ownServer: null }])
    expect(wrapper.get('[data-saved]').text()).toBe(client.readConfirmation())
  })

  it('saves the appearance picked', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)

    // Act
    await wrapper.get('[value="dark"]').setValue(true)
    await flushPromises()

    // Assert
    expect(client.savedForms.at(-1)?.appearance).toBe('dark')
  })

  it('waits for the whole address before saving an own server', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    const address = createAddress()
    await wrapper.get('#setting-servers').setValue('own')
    const server = wrapper.get('#setting-server')

    // Act
    await type(server, address.slice(0, 4))
    const whileTyping = client.savedForms.length
    await type(server, address)
    await server.trigger('change')
    await flushPromises()

    // Assert
    expect(whileTyping).toBe(0)
    expect(client.savedForms).toEqual([{ launchAtLogin: true, appearance: 'system', ownServer: address }])
  })

  it('keeps what was typed when the server is refused, and says why', async () => {
    // Arrange
    const refusal = faker.lorem.sentence()
    const address = createAddress()
    const client = new InMemoryNetworkQualityClient()
    client.refuseSettings(refusal)
    const wrapper = await mountSettings(client)
    await wrapper.get('#setting-servers').setValue('own')
    const server = wrapper.get('#setting-server')

    // Act
    await server.setValue(address)
    await server.trigger('change')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-refused]').text()).toBe(refusal)
    expect((wrapper.get('#setting-server').element as HTMLInputElement).value).toBe(address)
    expect(wrapper.get('#setting-server').attributes('aria-invalid')).toBe('true')
  })

  it('goes back to the public servers in one change', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    await wrapper.get('#setting-servers').setValue('own')
    await wrapper.get('#setting-server').setValue(createAddress())
    await wrapper.get('#setting-server').trigger('change')
    await flushPromises()

    // Act
    await wrapper.get('#setting-servers').setValue('default')
    await flushPromises()

    // Assert
    expect(client.savedForms.at(-1)?.ownServer).toBeNull()
    expect(wrapper.find('#setting-server').exists()).toBe(false)
  })
})

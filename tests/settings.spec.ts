import { faker } from '@faker-js/faker'
import { flushPromises, mount, type DOMWrapper } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SettingsView from '../src/components/SettingsView.vue'
import { InMemoryNetworkQualityClient, createWindowProvisions } from './fakes/in-memory-network-quality-client'

async function mountSettings(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(SettingsView, {
    global: { provide: createWindowProvisions(client) },
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
    expect(client.savedForms).toEqual([
      {
        launchAtLogin: false,
        appearance: 'system',
        ownServer: null,
        notifiesOnDegradation: true,
        allowsThroughputTests: false,
      },
    ])
    expect(wrapper.get('[data-saved]').text()).toBe(client.readConfirmation())
  })

  it('turns the degradation notifications off', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)

    // Act
    await wrapper.get('#setting-notify').setValue(false)
    await flushPromises()

    // Assert
    expect(client.savedForms.at(-1)?.notifiesOnDegradation).toBe(false)
  })

  it('shows the notifications as turned off once saved so', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    await client.saveSettings({
      launchAtLogin: true,
      appearance: 'system',
      ownServer: null,
      notifiesOnDegradation: false,
      allowsThroughputTests: false,
    })

    // Act
    const wrapper = await mountSettings(client)

    // Assert
    expect((wrapper.get('#setting-notify').element as HTMLInputElement).checked).toBe(false)
  })

  it('allows the throughput tests once switched on', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)

    // Act
    await wrapper.get('#setting-throughput').setValue(true)
    await flushPromises()

    // Assert
    expect(client.savedForms.at(-1)?.allowsThroughputTests).toBe(true)
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

  it('saves nothing while an address is being typed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    await wrapper.get('#setting-servers').setValue('own')

    // Act
    await type(wrapper.get('#setting-server'), createAddress().slice(0, 4))
    await flushPromises()

    // Assert
    expect(client.savedForms).toEqual([])
  })

  it('saves the own server once its address is complete', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    const address = createAddress()
    await wrapper.get('#setting-servers').setValue('own')
    const server = wrapper.get('#setting-server')
    await type(server, address)

    // Act
    await server.trigger('change')
    await flushPromises()

    // Assert
    expect(client.savedForms).toEqual([
      {
        launchAtLogin: true,
        appearance: 'system',
        ownServer: address,
        notifiesOnDegradation: true,
        allowsThroughputTests: false,
      },
    ])
  })

  it('saves the look with the last accepted server while an address is half typed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    await wrapper.get('#setting-servers').setValue('own')
    await type(wrapper.get('#setting-server'), createAddress().slice(0, 4))

    // Act
    await wrapper.get('[value="dark"]').setValue(true)
    await flushPromises()

    // Assert
    expect(client.savedForms).toEqual([
      {
        launchAtLogin: true,
        appearance: 'dark',
        ownServer: null,
        notifiesOnDegradation: true,
        allowsThroughputTests: false,
      },
    ])
    expect(wrapper.find('[data-refused]').exists()).toBe(false)
  })

  it('sends one save at a time, in the order of the changes', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSettings(client)
    client.holdSaves()
    await wrapper.get('#setting-launch').setValue(false)
    await wrapper.get('[value="light"]').setValue(true)
    await flushPromises()
    const whileTheFirstWaits = client.savedForms.length

    // Act
    client.releaseSaves()
    await flushPromises()
    client.releaseSaves()
    await flushPromises()

    // Assert
    expect(whileTheFirstWaits).toBe(1)
    expect(client.savedForms.map((form) => form.appearance)).toEqual(['system', 'light'])
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
    await type(server, address)
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
    await type(wrapper.get('#setting-server'), createAddress())
    await wrapper.get('#setting-server').trigger('change')
    await flushPromises()

    // Act
    await wrapper.get('#setting-servers').setValue('public')
    await flushPromises()

    // Assert
    expect(client.savedForms.at(-1)?.ownServer).toBeNull()
    expect(wrapper.find('#setting-server').exists()).toBe(false)
  })
})

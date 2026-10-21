import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import SettingsView from '../src/components/SettingsView.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import { InMemoryNetworkQualityClient } from './fakes/in-memory-network-quality-client'

async function mountSection(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(SettingsView, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
  await flushPromises()
  return wrapper
}

describe('SettingsView, data', () => {
  it('exports the chosen period and says where the files went', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-export-period]').setValue('all')

    // Act
    await wrapper.get('[data-export]').trigger('click')
    await flushPromises()

    // Assert
    expect(client.exports).toEqual(['all'])
    expect(wrapper.get('[data-data-status]').text()).toBe(client.exportConfirmation)
  })

  it('says what goes before erasing, and erases only once confirmed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-erase]').trigger('click')
    const warning = wrapper.get('[data-erase-warning]').text()
    const before = client.erases

    // Act
    await wrapper.get('[data-erase-confirm]').trigger('click')
    await flushPromises()

    // Assert
    expect(warning).toContain('mesures')
    expect(warning).toContain('tests de débit')
    expect(warning).toContain('réseaux connus')
    expect(warning).toContain('réglages sont conservés')
    expect(before).toBe(0)
    expect(client.erases).toBe(1)
    expect(wrapper.get('[data-data-status]').text()).toBe(client.eraseConfirmation)
    expect(wrapper.find('[data-erase-warning]').exists()).toBe(false)
  })

  it('erases nothing when the warning is turned down', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-erase]').trigger('click')

    // Act
    await wrapper.get('[data-erase-cancel]').trigger('click')

    // Assert
    expect(client.erases).toBe(0)
    expect(wrapper.find('[data-erase-warning]').exists()).toBe(false)
  })

  it('says why an export did not happen', async () => {
    // Arrange
    const refusal = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.refuseDataActions(refusal)
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-export]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-data-refused]').text()).toBe(refusal)
    expect(wrapper.find('[data-data-status]').text()).toBe('')
  })
})

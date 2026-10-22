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
  it('exports the chosen period into the chosen folder and offers to show it', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-export-period]').setValue('all')

    // Act
    await wrapper.get('[data-export]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-reveal-export]').trigger('click')

    // Assert
    expect(client.exports).toEqual(['all'])
    expect(wrapper.get('[data-data-status]').text()).toBe(client.exportConfirmation)
    expect(wrapper.get('[data-reveal-export]').text()).toBe(client.revealLabel)
    expect(client.exportRevealings).toBe(1)
  })

  it('exports nothing and says nothing when the folder dialog is closed without a choice', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.closeFolderDialog()
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-export]').trigger('click')
    await flushPromises()

    // Assert
    expect(client.exports).toEqual([])
    expect(wrapper.get('[data-data-status]').text()).toBe('')
    expect(wrapper.find('[data-reveal-export]').exists()).toBe(false)
  })

  it('shows the export running once the folder is chosen, and cannot start it twice', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.holdExports()
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-export]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-export]').trigger('click')

    // Assert
    expect(wrapper.get('[data-export]').text()).toBe('Export en cours…')
    expect(wrapper.get('[data-export]').attributes('disabled')).toBeDefined()
    expect(client.exports).toHaveLength(1)
    client.finishExport()
  })

  it('has nothing to export before the first measure', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.startWithEmptyHistory()

    // Act
    const wrapper = await mountSection(client)

    // Assert
    expect(wrapper.get('[data-export]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-export-unavailable]').text()).toBe(client.nothingToExport)
  })

  it('has nothing to export right after an erase', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-erase]').trigger('click')

    // Act
    await wrapper.get('[data-erase-confirm]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-export]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-export-unavailable]').text()).toBe(client.nothingToExport)
  })

  it('says why a history kept in memory cannot be exported', async () => {
    // Arrange
    const reason = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.keepHistoryInMemory(reason)

    // Act
    const wrapper = await mountSection(client)

    // Assert
    expect(wrapper.get('[data-export-unavailable]').text()).toBe(reason)
  })

  it('allows the export once measures exist, when the window comes back', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.startWithEmptyHistory()
    const wrapper = await mountSection(client)
    client.measureFirstMinute()

    // Act
    window.dispatchEvent(new Event('focus'))
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-export]').attributes('disabled')).toBeUndefined()
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

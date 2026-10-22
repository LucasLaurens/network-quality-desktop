import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import App from '../src/App.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createConnectionDisplay,
  createHelpDisplay,
  createThroughputPanel,
  createWindowModeTexts,
} from './fakes/in-memory-network-quality-client'

async function mountApp(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(App, { global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } } })
  await flushPromises()
  return wrapper
}

describe('App', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('shows the version reported by the Rust core', async () => {
    // Arrange
    const version = faker.system.semver()
    const client = new InMemoryNetworkQualityClient({ name: faker.word.noun(), version })

    // Act
    const wrapper = mount(App, { global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } } })
    await flushPromises()

    // Assert
    expect(wrapper.get('h1').text()).toBe('Qualité de connexion')
    expect(wrapper.text()).toContain(`Version ${version}`)
  })

  it('explains the missing dot when the desktop has no tray', async () => {
    // Arrange
    const hint = createWindowModeTexts()
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), hint)

    // Act
    const wrapper = await mountApp(client)

    // Assert
    expect(wrapper.get('[data-tray-hint]').text()).toContain(hint.message)
  })

  it('shows no note when the dot is visible', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()

    // Act
    const wrapper = await mountApp(client)

    // Assert
    expect(wrapper.find('[data-tray-hint]').exists()).toBe(false)
  })

  it('opens the extension page from the note', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), createWindowModeTexts())
    const wrapper = await mountApp(client)

    // Act
    await wrapper.get('[data-tray-hint-link]').trigger('click')

    // Assert
    expect(client.extensionPageOpenings).toBe(1)
  })

  it('keeps the note hidden once dismissed, across launches', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), createWindowModeTexts())
    const first = await mountApp(client)
    await first.get('[data-tray-hint-dismiss]').trigger('click')
    first.unmount()

    // Act
    const second = await mountApp(client)

    // Assert
    expect(second.find('[data-tray-hint]').exists()).toBe(false)
  })

  it('offers a way to quit in window mode, even once the note is dismissed', async () => {
    // Arrange
    const texts = createWindowModeTexts()
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), texts)
    const wrapper = await mountApp(client)
    await wrapper.get('[data-tray-hint-dismiss]').trigger('click')
    const quit = wrapper.get('[data-quit]')

    // Act
    await quit.trigger('click')

    // Assert
    expect(quit.text()).toBe(texts.quitLabel)
    expect(client.quitRequests).toBe(1)
  })

  it('shows the version even when the window mode cannot be read', async () => {
    // Arrange
    const version = faker.system.semver()
    const client = new InMemoryNetworkQualityClient({ name: faker.word.noun(), version })
    client.failWindowModeRead()

    // Act
    const wrapper = await mountApp(client)

    // Assert
    expect(wrapper.text()).toContain(`Version ${version}`)
  })

  it('says when the history had to be set aside', async () => {
    // Arrange
    const notice = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.setHistoryNotice(notice)

    // Act
    const wrapper = await mountApp(client)

    // Assert
    expect(wrapper.get('[data-history-notice]').text()).toBe(notice)
  })

  it('opens the settings page from the dashboard', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountApp(client)

    // Act
    await wrapper.get('[data-page="settings"]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.find('#settings-title').exists()).toBe(true)
    expect(wrapper.get('[data-page="settings"]').attributes('aria-current')).toBe('page')
  })

  it('goes back to the dashboard from the settings page', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountApp(client)
    await wrapper.get('[data-page="settings"]').trigger('click')
    await flushPromises()

    // Act
    await wrapper.get('[data-page="dashboard"]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.find('#settings-title').exists()).toBe(false)
    expect(wrapper.get('[data-page="dashboard"]').attributes('aria-current')).toBe('page')
  })

  it('opens the settings from the throughput section', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: false }))
    const wrapper = await mountApp(client)

    // Act
    await wrapper.get('[data-open-settings]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.find('#settings-title').exists()).toBe(true)
  })

  it('opens the settings page when the tray menu asks for it', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountApp(client)

    // Act
    client.sendDashboardRequest({ kind: 'settings' })
    await flushPromises()

    // Assert
    expect(wrapper.find('#settings-title').exists()).toBe(true)
  })

  it('runs on the dashboard the test the tray menu asked for, even from the settings page', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: true }))
    const wrapper = await mountApp(client)
    client.sendDashboardRequest({ kind: 'settings' })
    await flushPromises()

    // Act
    client.sendDashboardRequest({ kind: 'throughputTest', profile: 'full' })
    await flushPromises()

    // Assert
    expect(wrapper.find('#settings-title').exists()).toBe(false)
    expect(client.throughputRuns).toEqual(['full'])
  })

  it('opens the help on the dashboard when the tray menu asks for it', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ help: createHelpDisplay() }))
    const wrapper = await mountApp(client)
    client.sendDashboardRequest({ kind: 'settings' })
    await flushPromises()

    // Act
    client.sendDashboardRequest({ kind: 'help' })
    await flushPromises()

    // Assert
    const details = wrapper.get('[data-help]')
    expect((details.element as HTMLDetailsElement).open).toBe(true)
  })
})

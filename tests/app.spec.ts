import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it } from 'vitest'
import App from '../src/App.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createConnectionDisplay,
  createTrayHint,
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
    const hint = createTrayHint()
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
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), createTrayHint())
    const wrapper = await mountApp(client)

    // Act
    await wrapper.get('[data-tray-hint-link]').trigger('click')

    // Assert
    expect(client.extensionPageOpenings).toBe(1)
  })

  it('keeps the note hidden once dismissed, across launches', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), createTrayHint())
    const first = await mountApp(client)
    await first.get('[data-tray-hint-dismiss]').trigger('click')
    first.unmount()

    // Act
    const second = await mountApp(client)

    // Assert
    expect(second.find('[data-tray-hint]').exists()).toBe(false)
  })
})

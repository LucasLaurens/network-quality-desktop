import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import App from '../src/App.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import { InMemoryNetworkQualityClient } from './fakes/in-memory-network-quality-client'

describe('App', () => {
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
})

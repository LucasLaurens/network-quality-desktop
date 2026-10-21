import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ThroughputSection from '../src/components/ThroughputSection.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createThroughputPanel,
  createThroughputRow,
} from './fakes/in-memory-network-quality-client'

async function mountSection(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(ThroughputSection, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
  await flushPromises()
  return wrapper
}

describe('ThroughputSection', () => {
  it('points to the settings while the tests are not allowed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: false }))
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-open-settings]').trigger('click')

    // Assert
    expect(wrapper.emitted('openSettings')).toHaveLength(1)
    expect(wrapper.find('[data-run="quick"]').exists()).toBe(false)
  })

  it('runs a test on a click only, and lists it first', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const older = createThroughputRow()
    client.setThroughputPanel(createThroughputPanel({ tests: [older] }))
    const wrapper = await mountSection(client)
    const untouched = client.throughputRuns.length

    // Act
    await wrapper.get('[data-run="quick"]').trigger('click')
    await flushPromises()

    // Assert
    expect(untouched).toBe(0)
    expect(client.throughputRuns).toEqual(['quick'])
    const rows = wrapper.findAll('[data-test-row]')
    expect(rows[0]?.text()).toContain(client.readNextThroughputRow().download)
    expect(rows[1]?.text()).toContain(older.download)
  })

  it('shows the running test and blocks a second one', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.holdThroughputTests()
    const wrapper = await mountSection(client)
    await wrapper.get('[data-run="full"]').trigger('click')

    // Act
    await wrapper.get('[data-run="quick"]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-run="full"]').text()).toBe('Test en cours…')
    expect(wrapper.get('[data-run="quick"]').attributes('disabled')).toBeDefined()
    expect(client.throughputRuns).toEqual(['full'])
  })

  it('asks first where data counts, and runs only once confirmed', async () => {
    // Arrange
    const warning = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ quickWarning: warning }))
    const wrapper = await mountSection(client)
    await wrapper.get('[data-run="quick"]').trigger('click')
    const shown = wrapper.get('[data-warning]').text()
    const before = client.throughputRuns.length

    // Act
    await wrapper.get('[data-confirm]').trigger('click')
    await flushPromises()

    // Assert
    expect(shown).toContain(warning)
    expect(before).toBe(0)
    expect(client.throughputRuns).toEqual(['quick'])
  })

  it('runs nothing when the warning is turned down', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ fullWarning: faker.lorem.sentence() }))
    const wrapper = await mountSection(client)
    await wrapper.get('[data-run="full"]').trigger('click')

    // Act
    await wrapper.get('[data-cancel]').trigger('click')

    // Assert
    expect(client.throughputRuns).toEqual([])
    expect(wrapper.find('[data-warning]').exists()).toBe(false)
  })

  it('says why a test did not run', async () => {
    // Arrange
    const refusal = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.refuseThroughputTests(refusal)
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-run="quick"]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-refused]').text()).toBe(refusal)
    expect(wrapper.get('[data-run="quick"]').attributes('disabled')).toBeUndefined()
  })
})

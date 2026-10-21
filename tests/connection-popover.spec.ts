import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ConnectionPopover from '../src/components/ConnectionPopover.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createConnectionDisplay,
  createMetricDisplay,
} from './fakes/in-memory-network-quality-client'

async function mountPopover(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(ConnectionPopover, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
  await flushPromises()
  return wrapper
}

describe('ConnectionPopover', () => {
  it('shows the state, the cause, the action and the detail it is given', async () => {
    // Arrange
    const display = createConnectionDisplay({
      state: 'offline',
      action: faker.lorem.sentence(),
      detail: faker.lorem.words(3),
    })
    const client = new InMemoryNetworkQualityClient(undefined, display)

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.get('h1').text()).toBe(display.stateLabel)
    expect(wrapper.text()).toContain(display.cause)
    expect(wrapper.text()).toContain(display.action)
    expect(wrapper.text()).toContain(display.detail)
    expect(wrapper.get('[data-state]').attributes('data-state')).toBe('offline')
  })

  it('names the network and how it is reached', async () => {
    // Arrange
    const network = { label: faker.company.name(), connection: faker.lorem.words(2) }
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    const where = wrapper.get('[data-network]')
    expect(where.text()).toContain(network.label)
    expect(where.text()).toContain(network.connection)
  })

  it('names no network without a default route', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network: null }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.find('[data-network]').exists()).toBe(false)
  })

  it('shows each metric with its value, its method and its gauge marker', async () => {
    // Arrange
    const label = faker.lorem.sentence()
    const metric = createMetricDisplay({ gauge: { position: 0.45, unstableFrom: 0.25, degradedAbove: 0.75, label } })
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ metrics: [metric] }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    const row = wrapper.get('[data-metric]')
    expect(row.text()).toContain(metric.name)
    expect(row.text()).toContain(metric.method)
    expect(row.text()).toContain(metric.value)
    expect(row.get('[data-marker]').attributes('style')).toContain('left: 45%')
    expect(row.findAll('[data-zone]')).toHaveLength(3)
    expect(row.get('[role="img"]').attributes('aria-label')).toBe(label)
  })

  it('shows a dash and no gauge marker for a metric not measured yet', async () => {
    // Arrange
    const metric = createMetricDisplay({ value: null, gauge: null })
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ metrics: [metric] }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.get('[data-metric]').text()).toContain('–')
    expect(wrapper.find('[data-marker]').exists()).toBe(false)
    expect(wrapper.findAll('[data-zone]')).toHaveLength(0)
  })

  it('keeps a display pushed while the first read was in flight', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay())
    client.holdReads()
    const wrapper = await mountPopover(client)
    const fresh = createConnectionDisplay()

    // Act
    client.pushDisplay(fresh)
    client.releaseReads()
    await flushPromises()

    // Assert
    expect(wrapper.get('h1').text()).toBe(fresh.stateLabel)
  })

  it('shows the hint of a metric close to its threshold', async () => {
    // Arrange
    const hint = faker.lorem.words(3)
    const client = new InMemoryNetworkQualityClient(
      undefined,
      createConnectionDisplay({ metrics: [createMetricDisplay({ hint })] }),
    )

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.get('[data-metric]').text()).toContain(hint)
  })

  it('follows every new display pushed by the core', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountPopover(client)
    const next = createConnectionDisplay({ state: 'degraded' })

    // Act
    client.pushDisplay(next)
    await flushPromises()

    // Assert
    expect(wrapper.get('h1').text()).toBe(next.stateLabel)
    expect(wrapper.text()).toContain(next.cause)
  })

  it('opens the dashboard from its main button', async () => {
    // Arrange
    const display = createConnectionDisplay()
    const client = new InMemoryNetworkQualityClient(undefined, display)
    const wrapper = await mountPopover(client)
    const button = wrapper.get('button[data-primary]')

    // Act
    await button.trigger('click')

    // Assert
    expect(button.text()).toBe(display.openDashboardLabel)
    expect(client.dashboardOpenings).toBe(1)
  })

  it('closes when Escape is pressed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    await mountPopover(client)

    // Act
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))

    // Assert
    expect(client.popoverClosings).toBe(1)
  })

  it('stops listening once closed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountPopover(client)

    // Act
    wrapper.unmount()

    // Assert
    expect(client.listenerCount).toBe(0)
  })
})

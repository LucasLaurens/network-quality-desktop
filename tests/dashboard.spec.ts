import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DashboardView from '../src/components/DashboardView.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createConnectionDisplay,
  createDashboardSummary,
  createEvolutionDisplay,
  createMetricDisplay,
  createNetworkRowDisplay,
  createOverviewDisplay,
} from './fakes/in-memory-network-quality-client'

async function mountDashboard(client: InMemoryNetworkQualityClient) {
  const wrapper = mount(DashboardView, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
  await flushPromises()
  return wrapper
}

describe('DashboardView', () => {
  it('answers first: the state, its cause and the facts of the day', async () => {
    // Arrange
    const display = createConnectionDisplay({ state: 'unstable', action: faker.lorem.sentence() })
    const overview = createOverviewDisplay()
    const client = new InMemoryNetworkQualityClient(undefined, display)
    client.setDashboardSummary(createDashboardSummary({ overview }))

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    expect(wrapper.get('#dashboard-state').text()).toBe(display.stateLabel)
    expect(wrapper.text()).toContain(display.cause)
    expect(wrapper.get('[data-action]').text()).toBe(display.action)
    expect(wrapper.get('[data-fact="today"]').text()).toContain(overview.today)
    expect(wrapper.get('[data-coverage]').text()).toBe(overview.todayCoverage)
    expect(wrapper.get('[data-fact="outages"]').text()).toBe(overview.outages)
    expect(wrapper.get('[data-fact="worst"]').text()).toBe(overview.worstMoment)
    expect(wrapper.get('[data-fact="since"]').text()).toBe(overview.onNetworkSince)
  })

  it('explains each current metric with its thresholds and its method', async () => {
    // Arrange
    const metric = createMetricDisplay({ hint: faker.lorem.words(3) })
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ metrics: [metric] }))

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    const shown = wrapper.get('[data-metric]')
    expect(shown.text()).toContain(metric.definition)
    expect(shown.text()).toContain(metric.method)
    expect(shown.get('[data-thresholds]').text()).toBe(metric.gauge?.label)
    expect(shown.get('[data-hint]').text()).toBe(metric.hint)
  })

  it('charts the last hour first, then the period picked', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountDashboard(client)

    // Act
    await wrapper.get('[data-option="week"]').trigger('click')
    await flushPromises()

    // Assert
    expect(client.evolutionRequests).toEqual(['hour', 'week'])
    expect(wrapper.get('[data-option="week"]').attributes('aria-pressed')).toBe('true')
  })

  it('switches the metric without reading the history again', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setEvolution(createEvolutionDisplay([10, 20, 30]))
    const wrapper = await mountDashboard(client)
    const latencyLine = wrapper.get('[data-line]').attributes('points')

    // Act
    await wrapper.get('[data-option="jitter"]').trigger('click')

    // Assert
    expect(wrapper.get('[data-line]').attributes('points')).not.toBe(latencyLine)
    expect(client.evolutionRequests).toEqual(['hour'])
  })

  it('breaks the line where nothing was measured, without bridging the gap', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setEvolution(createEvolutionDisplay([10, 20, null, null, 30, 40]))

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    expect(wrapper.findAll('[data-line]')).toHaveLength(2)
  })

  it('marks the changes of network on the chart', async () => {
    // Arrange
    const label = faker.lorem.words(3)
    const evolution = { ...createEvolutionDisplay(), networkChanges: [{ position: 0.5, label }] }
    const client = new InMemoryNetworkQualityClient()
    client.setEvolution(evolution)

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    expect(wrapper.get('[data-network-change]').text()).toBe(label)
  })

  it('names the value under the pointer, read from the keyboard too', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const evolution = createEvolutionDisplay([10, 20, 30])
    client.setEvolution(evolution)
    const wrapper = await mountDashboard(client)

    // Act
    await wrapper.get('[data-chart]').trigger('keydown', { key: 'ArrowLeft' })

    // Assert
    const tooltip = wrapper.get('[data-tooltip]')
    expect(tooltip.text()).toContain(evolution.times[1])
    expect(tooltip.text()).toContain(evolution.latency.texts[1])
  })

  it('flags the current network and the most reliable one', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const summaryNetworks = [
      createNetworkRowDisplay({ isMostReliable: true }),
      createNetworkRowDisplay({ isCurrent: true }),
    ]
    client.setDashboardSummary(createDashboardSummary({ networks: summaryNetworks }))

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    const rows = wrapper.findAll('[data-network-row]')
    expect(rows[0]?.get('[data-measured]').text()).toBe(summaryNetworks[0]?.measured)
    expect(rows[0]?.find('[data-most-reliable]').exists()).toBe(true)
    expect(rows[0]?.find('[data-current]').exists()).toBe(false)
    expect(rows[1]?.find('[data-current]').exists()).toBe(true)
  })

  it('says what will appear on a first launch', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setDashboardSummary(createDashboardSummary({ networks: [] }))
    client.setEvolution(createEvolutionDisplay([null, null]))

    // Act
    const wrapper = await mountDashboard(client)

    // Assert
    expect(wrapper.find('[data-chart-empty]').exists()).toBe(true)
    expect(wrapper.find('[data-chart]').exists()).toBe(false)
    expect(wrapper.find('[data-networks-empty]').exists()).toBe(true)
  })
})

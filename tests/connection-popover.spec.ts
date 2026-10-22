import { faker } from '@faker-js/faker'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ConnectionPopover from '../src/components/ConnectionPopover.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import {
  InMemoryNetworkQualityClient,
  createConnectionDisplay,
  createHelpDisplay,
  createMetricDisplay,
  createPathDisplay,
  createSegmentDisplay,
  createThroughputPanel,
  createThroughputRow,
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
    const network = {
      label: faker.company.name(),
      connection: faker.lorem.words(2),
      wifiDetails: null,
      revealLabel: null,
    }
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    const where = wrapper.get('[data-network]')
    expect(where.text()).toContain(network.label)
    expect(where.text()).toContain(network.connection)
  })

  it('shows the details of the Wi-Fi link', async () => {
    // Arrange
    const wifiDetails = faker.lorem.words(4)
    const network = { label: faker.lorem.words(2), connection: faker.lorem.word(), wifiDetails, revealLabel: null }
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.get('[data-wifi-details]').text()).toBe(wifiDetails)
  })

  it('offers to reveal a hidden Wi-Fi name', async () => {
    // Arrange
    const revealLabel = faker.lorem.words(3)
    const network = { label: faker.lorem.words(2), connection: faker.lorem.word(), wifiDetails: null, revealLabel }
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network }))
    const wrapper = await mountPopover(client)
    const button = wrapper.get('[data-reveal-name]')

    // Act
    await button.trigger('click')

    // Assert
    expect(button.text()).toBe(revealLabel)
    expect(client.nameRevealRequests).toBe(1)
  })

  it('names no network without a default route', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ network: null }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.find('[data-network]').exists()).toBe(false)
  })

  it('draws the path and names the segment at fault with a word', async () => {
    // Arrange
    const fault = faker.lorem.word()
    const path = createPathDisplay({ internetSegment: createSegmentDisplay({ fault, state: 'degraded' }) })
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ path }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    const drawn = wrapper.get('[data-path]')
    expect(drawn.attributes('aria-label')).toBe(path.description)
    expect(drawn.text()).toContain(path.computer)
    expect(drawn.text()).toContain(path.gateway)
    expect(drawn.get('[data-segment="internet"]').text()).toContain(fault)
    expect(drawn.find('[data-segment="local"] [data-fault]').exists()).toBe(false)
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

  it('shows the wording of the core and no gauge marker for a metric not measured yet', async () => {
    // Arrange
    const pending = faker.lorem.words(2)
    const metric = createMetricDisplay({ value: pending, gauge: null })
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ metrics: [metric] }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    expect(wrapper.get('[data-metric]').text()).toContain(pending)
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

  it('points to the settings while the throughput tests are not allowed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: false }))
    const wrapper = await mountPopover(client)

    // Act
    await wrapper.get('[data-open-settings]').trigger('click')

    // Assert
    expect(client.settingsOpenings).toBe(1)
    expect(wrapper.find('[data-run="quick"]').exists()).toBe(false)
  })

  it('shows the last test and runs a new one on a click', async () => {
    // Arrange
    const last = createThroughputRow()
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: true, tests: [last] }))
    const wrapper = await mountPopover(client)
    const shownBefore = wrapper.get('[data-last-test]').text()

    // Act
    await wrapper.get('[data-run="quick"]').trigger('click')
    await flushPromises()

    // Assert
    expect(shownBefore).toContain(`réception ${last.download}`)
    expect(shownBefore).toContain(`envoi ${last.upload}`)
    expect(client.throughputRuns).toEqual(['quick'])
    expect(wrapper.get('[data-last-test]').text()).toContain(client.readNextThroughputRow().download)
  })

  it('asks first where data counts, as the dashboard does', async () => {
    // Arrange
    const warning = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: true }))
    client.warnBeforeThroughputTests(warning)
    const wrapper = await mountPopover(client)
    await wrapper.get('[data-run="full"]').trigger('click')
    await flushPromises()

    // Act
    await wrapper.get('[data-confirm]').trigger('click')
    await flushPromises()

    // Assert
    expect(client.throughputRuns).toEqual(['full'])
  })

  it('reads the tests again each time it comes back, as the setting may have changed', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: false }))
    const wrapper = await mountPopover(client)
    client.setThroughputPanel(createThroughputPanel({ isAllowed: true }))

    // Act
    window.dispatchEvent(new Event('focus'))
    await flushPromises()

    // Assert
    expect(wrapper.find('[data-run="quick"]').exists()).toBe(true)
  })

  it('shows the running test, blocks a second one, and says why a test did not run', async () => {
    // Arrange
    const refusal = faker.lorem.sentence()
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ isAllowed: true }))
    client.holdThroughputTests()
    const wrapper = await mountPopover(client)
    await wrapper.get('[data-run="quick"]').trigger('click')
    const busyLabel = wrapper.get('[data-run="quick"]').text()
    const isFullBlocked = wrapper.get('[data-run="full"]').attributes('disabled') !== undefined
    client.refuseThroughputTests(refusal)

    // Act
    client.finishThroughputTest()
    await flushPromises()

    // Assert
    expect(busyLabel).toBe('Test en cours…')
    expect(isFullBlocked).toBe(true)
    expect(wrapper.get('[data-refused]').text()).toBe(refusal)
  })

  it('offers folded help when the system refuses the ping', async () => {
    // Arrange
    const help = createHelpDisplay()
    const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay({ help }))

    // Act
    const wrapper = await mountPopover(client)

    // Assert
    const details = wrapper.get('[data-help]')
    expect(details.get('summary').text()).toBe(help.title)
    expect((details.element as HTMLDetailsElement).open).toBe(false)
    expect(details.findAll('code').map((code) => code.text())).toEqual(help.steps.map((step) => step.command))
  })
})

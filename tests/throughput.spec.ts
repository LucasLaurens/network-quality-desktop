import { faker } from '@faker-js/faker'
import { flushPromises } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ThroughputPanel } from '../src/bindings'
import ThroughputSection from '../src/components/ThroughputSection.vue'
import {
  InMemoryNetworkQualityClient,
  createThroughputPanel,
  createThroughputRow,
  mountWindow,
} from './fakes/in-memory-network-quality-client'

function mountSection(client: InMemoryNetworkQualityClient) {
  return mountWindow(ThroughputSection, client)
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
    client.warnBeforeThroughputTests(warning)
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
    client.warnBeforeThroughputTests(faker.lorem.sentence())
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

  it('reads the tests again when the window comes back, after a test run elsewhere', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = await mountSection(client)
    const newer = createThroughputRow()
    client.setThroughputPanel(createThroughputPanel({ tests: [newer] }))

    // Act
    window.dispatchEvent(new Event('focus'))
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-test-row]').text()).toContain(newer.download)
  })

  it('shows no pager while every test fits one page', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPanel(createThroughputPanel({ tests: [createThroughputRow()] }))

    // Act
    const wrapper = await mountSection(client)

    // Assert
    expect(wrapper.find('[data-pager]').exists()).toBe(false)
  })

  it('starts on the newest tests, with only the way to older ones open', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const [first, second] = createTwoPages()
    client.setThroughputPages([first, second])

    // Act
    const wrapper = await mountSection(client)

    // Assert
    expect(wrapper.get('[data-page-label]').text()).toBe(first.pageLabel)
    expect(wrapper.get('[data-newer]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-older]').attributes('disabled')).toBeUndefined()
  })

  it('shows the older tests on the next page, and says which page it is', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const [first, second] = createTwoPages()
    client.setThroughputPages([first, second])
    const wrapper = await mountSection(client)

    // Act
    await wrapper.get('[data-older]').trigger('click')
    await flushPromises()

    // Assert
    expect(wrapper.get('[data-test-row]').text()).toContain(second.tests[0]?.download)
    expect(wrapper.get('[data-page-label]').text()).toBe(second.pageLabel)
    expect(wrapper.get('[data-page-label]').attributes('aria-live')).toBe('polite')
    expect(wrapper.get('[data-older]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-newer]').attributes('disabled')).toBeUndefined()
  })

  it('stays on its page when the window comes back', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    client.setThroughputPages(createTwoPages())
    const wrapper = await mountSection(client)
    await wrapper.get('[data-older]').trigger('click')
    await flushPromises()

    // Act
    window.dispatchEvent(new Event('focus'))
    await flushPromises()

    // Assert
    expect(client.throughputPageReads.at(-1)).toBe(1)
  })

  it('goes back to the first page after a test run from the section', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const [first, second] = createTwoPages()
    client.setThroughputPages([first, second])
    const wrapper = await mountSection(client)
    await wrapper.get('[data-older]').trigger('click')
    await flushPromises()

    // Act
    await wrapper.get('[data-run="quick"]').trigger('click')
    await flushPromises()

    // Assert
    expect(client.throughputPageReads.at(-1)).toBe(0)
    expect(wrapper.get('[data-page-label]').text()).toBe(first.pageLabel)
    expect(wrapper.get('[data-test-row]').text()).toContain(client.readNextThroughputRow().download)
  })
})

function createTwoPages(): [ThroughputPanel, ThroughputPanel] {
  return [
    createThroughputPanel({
      tests: [createThroughputRow()],
      page: 0,
      pageCount: 2,
      pageLabel: faker.lorem.words(3),
    }),
    createThroughputPanel({
      tests: [createThroughputRow()],
      page: 1,
      pageCount: 2,
      pageLabel: faker.lorem.words(3),
    }),
  ]
}

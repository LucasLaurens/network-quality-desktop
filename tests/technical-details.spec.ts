import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TechnicalDetails from '../src/components/TechnicalDetails.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from '../src/services/network-quality-client'
import { InMemoryNetworkQualityClient, createDetailSection } from './fakes/in-memory-network-quality-client'

function mountDetails(client: InMemoryNetworkQualityClient) {
  return mount(TechnicalDetails, {
    global: { provide: { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client } },
  })
}

async function toggle(wrapper: ReturnType<typeof mountDetails>, isOpen: boolean): Promise<void> {
  // The element fires its own toggle event, as on a click.
  ;(wrapper.get('details').element as HTMLDetailsElement).open = isOpen
  await vi.advanceTimersByTimeAsync(0)
  await flushPromises()
}

describe('TechnicalDetails', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('reads nothing while folded, then the details and the line once opened', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = mountDetails(client)
    await flushPromises()
    const readsWhileFolded = client.technicalDetailsReads + client.lineDetailsReads

    // Act
    await toggle(wrapper, true)

    // Assert
    expect(readsWhileFolded).toBe(0)
    const titles = wrapper.findAll('[data-detail-section] h3').map((title) => title.text())
    expect(titles).toEqual([...client.technicalDetails.sections, client.lineDetails.section].map((s) => s.title))
    expect(wrapper.get('[data-line-note]').text()).toBe(client.lineDetails.note)
  })

  it('reads the traffic again while open, and stops once folded', async () => {
    // Arrange
    const client = new InMemoryNetworkQualityClient()
    const wrapper = mountDetails(client)
    await toggle(wrapper, true)
    const newer = createDetailSection()
    const refreshMs = client.technicalDetails.refreshSeconds * 1000
    client.technicalDetails = { ...client.technicalDetails, sections: [newer] }

    // Act
    await vi.advanceTimersByTimeAsync(refreshMs)
    const readsWhileOpen = client.technicalDetailsReads
    await toggle(wrapper, false)
    await vi.advanceTimersByTimeAsync(3 * refreshMs)

    // Assert
    expect(readsWhileOpen).toBe(2)
    expect(client.technicalDetailsReads).toBe(2)
    expect(client.lineDetailsReads).toBe(1)
    expect(wrapper.text()).toContain(newer.title)
  })
})

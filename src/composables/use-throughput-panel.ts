import { ref, type Ref } from 'vue'
import type { ThroughputPanel } from '../bindings'
import type { NetworkQualityClient } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import { useLoadOnFocus } from './use-load-on-focus'

const FIRST_PAGE = 0

/** The throughput tests, one page at a time, read again on the same page each time the window
 * comes back to the front. */
export function useThroughputPanel(client: NetworkQualityClient): {
  panel: Ref<ThroughputPanel | null>
  loadPanel: () => Promise<void>
  showPage: (page: number) => Promise<void>
  showFirstPage: () => Promise<void>
} {
  const panel = ref<ThroughputPanel | null>(null)
  const showPage = async (page: number): Promise<void> => {
    try {
      panel.value = await client.readThroughputPanel(readUtcOffsetSeconds(), page)
    } catch (error) {
      console.error('Reading the throughput tests failed', error)
    }
  }
  const loadPanel = (): Promise<void> => showPage(panel.value?.page ?? FIRST_PAGE)
  const showFirstPage = (): Promise<void> => showPage(FIRST_PAGE)
  useLoadOnFocus(loadPanel)
  return { panel, loadPanel, showPage, showFirstPage }
}

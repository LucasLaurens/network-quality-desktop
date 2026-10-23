import { ref, type Ref } from 'vue'
import type { ThroughputPanel } from '../bindings'
import type { NetworkQualityClient } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import { useLoadOnFocus } from './use-load-on-focus'

/** The throughput tests, read again each time the window comes back to the front. */
export function useThroughputPanel(client: NetworkQualityClient): {
  panel: Ref<ThroughputPanel | null>
  loadPanel: () => Promise<void>
} {
  const panel = ref<ThroughputPanel | null>(null)
  const loadPanel = async (): Promise<void> => {
    try {
      panel.value = await client.readThroughputPanel(readUtcOffsetSeconds())
    } catch (error) {
      console.error('Reading the throughput tests failed', error)
    }
  }
  useLoadOnFocus(loadPanel)
  return { panel, loadPanel }
}

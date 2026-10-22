// Development-only page: renders the popover with prepared displays, for visual checks in a browser.
import { createApp } from 'vue'
import ConnectionPopover from '../../src/components/ConnectionPopover.vue'
import '../../src/styles/tokens.css'
import { INTERFACE_TEXTS_KEY } from '../../src/composables/interface-texts'
import { NETWORK_QUALITY_CLIENT_KEY } from '../../src/services/network-quality-client'
import interfaceTexts from '../fakes/interface-texts.json'
import {
  InMemoryNetworkQualityClient,
  createThroughputPanel,
  createThroughputRow,
} from '../fakes/in-memory-network-quality-client'
import { NBSP, connectionScenarios } from './connection-scenarios'

// `?tests=on` shows the throughput tests allowed, with a last result.
const parameters = new URLSearchParams(window.location.search)
const scenario = parameters.get('scenario') ?? 'degraded'
const client = new InMemoryNetworkQualityClient(
  undefined,
  connectionScenarios[scenario] ?? connectionScenarios.degraded,
)
client.setThroughputPanel(
  parameters.get('tests') === 'on'
    ? createThroughputPanel({
        isAllowed: true,
        tests: [
          createThroughputRow({
            when: `22/10, 9${NBSP}h${NBSP}41`,
            download: `412${NBSP}Mbit/s`,
            upload: `96${NBSP}Mbit/s`,
          }),
        ],
      })
    : createThroughputPanel({ isAllowed: false }),
)
createApp(ConnectionPopover)
  .provide(NETWORK_QUALITY_CLIENT_KEY, client)
  .provide(INTERFACE_TEXTS_KEY, interfaceTexts)
  .mount('#app')

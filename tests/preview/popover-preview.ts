// Development-only page: renders the popover with prepared displays, for visual checks in a browser.
import { createApp } from 'vue'
import ConnectionPopover from '../../src/components/ConnectionPopover.vue'
import '../../src/styles/tokens.css'
import { NETWORK_QUALITY_CLIENT_KEY } from '../../src/services/network-quality-client'
import { InMemoryNetworkQualityClient } from '../fakes/in-memory-network-quality-client'
import { connectionScenarios } from './connection-scenarios'

const scenario = new URLSearchParams(window.location.search).get('scenario') ?? 'degraded'
const client = new InMemoryNetworkQualityClient(
  undefined,
  connectionScenarios[scenario] ?? connectionScenarios.degraded,
)
createApp(ConnectionPopover).provide(NETWORK_QUALITY_CLIENT_KEY, client).mount('#app')

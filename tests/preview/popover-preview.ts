// Development-only page: renders the popover with prepared displays, for visual checks in a browser.
import { createApp } from 'vue'
import ConnectionPopover from '../../src/components/ConnectionPopover.vue'
import '../../src/styles/tokens.css'
import { NETWORK_QUALITY_CLIENT_KEY } from '../../src/services/network-quality-client'
import { InMemoryNetworkQualityClient, createConnectionDisplay } from '../fakes/in-memory-network-quality-client'
import type { ConnectionDisplay } from '../../src/bindings'

const NBSP = ' '
const scenarios: Record<string, ConnectionDisplay> = {
  degraded: createConnectionDisplay({
    state: 'unstable',
    stateLabel: 'Instable',
    cause: 'La connexion ne tient pas les seuils de la visio.',
    detail: `Latence 180${NBSP}ms, seuil 100${NBSP}ms`,
    metrics: [
      {
        name: 'Latence',
        method: `p90, 15${NBSP}s`,
        value: `180${NBSP}ms`,
        state: 'unstable',
        gauge: { position: 0.45, unstableFrom: 0.25, degradedAbove: 0.75 },
        hint: null,
      },
      {
        name: 'Gigue',
        method: `15${NBSP}s`,
        value: `24${NBSP}ms`,
        state: 'solid',
        gauge: { position: 0.34, unstableFrom: 0.43, degradedAbove: 0.71 },
        hint: 'Proche du seuil',
      },
      {
        name: 'Perte',
        method: `60${NBSP}s`,
        value: `0,4${NBSP}%`,
        state: 'solid',
        gauge: { position: 0.05, unstableFrom: 0.125, degradedAbove: 0.625 },
        hint: null,
      },
    ],
  }),
  offline: createConnectionDisplay({
    state: 'offline',
    stateLabel: 'Hors ligne',
    cause: 'Aucun réseau connecté.',
    action: 'Se connecter à un Wi-Fi ou brancher un câble.',
    metrics: ['Latence', 'Gigue', 'Perte'].map((name) => ({
      name,
      method: `15${NBSP}s`,
      value: null,
      state: null,
      gauge: null,
      hint: null,
    })),
  }),
}

const scenario = new URLSearchParams(window.location.search).get('scenario') ?? 'degraded'
const client = new InMemoryNetworkQualityClient(undefined, scenarios[scenario] ?? scenarios.degraded)
createApp(ConnectionPopover).provide(NETWORK_QUALITY_CLIENT_KEY, client).mount('#app')

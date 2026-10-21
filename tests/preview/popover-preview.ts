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
    openDashboardLabel: 'Ouvrir le tableau de bord',
    network: { label: 'Réseau Wi-Fi', connection: 'Wi-Fi, via un VPN' },
    state: 'unstable',
    stateLabel: 'Instable',
    cause: 'La connexion ne tient pas les seuils de la visio.',
    detail: `Latence 180${NBSP}ms, seuil 100${NBSP}ms`,
    metrics: [
      {
        name: 'Latence',
        method: `p90, 15${NBSP}s`,
        value: `180${NBSP}ms`,
        gauge: {
          position: 0.45,
          unstableFrom: 0.25,
          degradedAbove: 0.75,
          label: `Solide sous 100${NBSP}ms, Dégradée au-delà de 300${NBSP}ms`,
        },
        hint: null,
      },
      {
        name: 'Gigue',
        method: `15${NBSP}s`,
        value: `24${NBSP}ms`,
        gauge: {
          position: 0.34,
          unstableFrom: 0.43,
          degradedAbove: 0.71,
          label: `Solide sous 30${NBSP}ms, Dégradée au-delà de 50${NBSP}ms`,
        },
        hint: 'Proche du seuil',
      },
      {
        name: 'Perte',
        method: `60${NBSP}s`,
        value: `0,4${NBSP}%`,
        gauge: {
          position: 0.05,
          unstableFrom: 0.125,
          degradedAbove: 0.625,
          label: `Solide sous 1${NBSP}%, Dégradée au-delà de 5${NBSP}%`,
        },
        hint: null,
      },
    ],
  }),
  offline: createConnectionDisplay({
    openDashboardLabel: 'Ouvrir le tableau de bord',
    network: { label: 'Réseau Wi-Fi', connection: 'Wi-Fi' },
    state: 'offline',
    stateLabel: 'Hors ligne',
    cause: 'Aucun serveur ne répond.',
    action: 'Vérifier le Wi-Fi ou le câble.',
    metrics: ['Latence', 'Gigue', 'Perte'].map((name) => ({
      name,
      method: `15${NBSP}s`,
      value: null,
      gauge: null,
      hint: null,
    })),
  }),
}

const scenario = new URLSearchParams(window.location.search).get('scenario') ?? 'degraded'
const client = new InMemoryNetworkQualityClient(undefined, scenarios[scenario] ?? scenarios.degraded)
createApp(ConnectionPopover).provide(NETWORK_QUALITY_CLIENT_KEY, client).mount('#app')

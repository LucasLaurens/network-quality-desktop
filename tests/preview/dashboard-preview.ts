// Development-only page: renders the dashboard with prepared data, for visual checks in a browser.
import { createApp } from 'vue'
import App from '../../src/App.vue'
import '../../src/styles/tokens.css'
import type { DashboardSummaryDisplay, DisplayState, EvolutionDisplay, SeriesDisplay } from '../../src/bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../../src/services/network-quality-client'
import { InMemoryNetworkQualityClient } from '../fakes/in-memory-network-quality-client'
import { NBSP, connectionScenarios } from './connection-scenarios'

// `?scenario=window` shows the window mode note, as on a desktop without a tray;
// `?scenario=first` shows a first launch, before any minute is kept.
const scenario = new URLSearchParams(window.location.search).get('scenario')
const windowModeTexts =
  scenario === 'window'
    ? {
        message:
          "Ce bureau n'affiche pas les icônes de la barre système : l'app reste ouverte dans une fenêtre. L'extension AppIndicator fait apparaître la pastille.",
        linkLabel: "Ouvrir la page de l'extension",
        dismissLabel: 'Masquer',
        quitLabel: 'Quitter',
      }
    : null

const POINTS = 60
const GAP = { from: 35, to: 39 }
const ms = (value: number): string => `${String(Math.round(value))}${NBSP}ms`
const percent = (value: number): string => `${value.toFixed(1).replace('.', ',')}${NBSP}%`

/** A calm line with a rough patch, then a gap where the computer slept. */
const latencies = Array.from({ length: POINTS }, (_, index) => {
  if (index >= GAP.from && index < GAP.to) return null
  const wave = 28 + 6 * Math.sin(index / 3) + 3 * Math.sin(index * 1.7)
  return index > 44 && index < 50 ? wave + 140 + 30 * Math.sin(index) : wave
})

const toState = (latency: number | null): DisplayState | null =>
  latency === null ? null : latency > 300 ? 'degraded' : latency >= 100 ? 'unstable' : 'solid'

const createSeries = (
  values: (number | null)[],
  format: (value: number) => string,
  thresholds: [number, number],
): SeriesDisplay => ({
  values,
  texts: values.map((value) => (value === null ? null : format(value))),
  unstableFrom: thresholds[0],
  degradedAbove: thresholds[1],
  unstableLabel: `Instable ${format(thresholds[0])}`,
  degradedLabel: `Dégradée ${format(thresholds[1])}`,
})

const start = Date.now() - 3600 * 1000
const evolution: EvolutionDisplay = {
  times: latencies.map((_, index) => {
    const time = new Date(start + index * 60 * 1000)
    return `${String(time.getHours())}${NBSP}h${NBSP}${String(time.getMinutes()).padStart(2, '0')}`
  }),
  states: latencies.map(toState),
  latency: createSeries(latencies, ms, [100, 300]),
  jitter: createSeries(
    latencies.map((latency) => (latency === null ? null : latency / 6)),
    ms,
    [30, 50],
  ),
  loss: createSeries(
    latencies.map((latency) => (latency === null ? null : latency / 120)),
    percent,
    [1, 5],
  ),
  networkChanges: [{ position: 0.62, label: `Changement de réseau${NBSP}: Livebox-4F2A` }],
  stateLabels: [
    { state: 'solid', label: 'Solide' },
    { state: 'unstable', label: 'Instable' },
    { state: 'degraded', label: 'Dégradée' },
    { state: 'offline', label: 'Hors ligne' },
  ],
  coverage: `Mesuré 93${NBSP}% de la période`,
  isEmpty: false,
}

const summary: DashboardSummaryDisplay = {
  overview: {
    today: `Solide 84${NBSP}% du temps`,
    todayCoverage: `mesuré 8${NBSP}h 40${NBSP}min sur 9${NBSP}h 12${NBSP}min`,
    outages: `1, à 13${NBSP}h${NBSP}48 (2${NBSP}min)`,
    worstMoment: `15${NBSP}h${NBSP}02, latence 210${NBSP}ms`,
    onNetworkSince: `3${NBSP}h 12${NBSP}min`,
  },
  networks: [
    {
      label: 'Ethernet bureau',
      connection: 'Ethernet',
      solid: 96,
      unstable: 4,
      degraded: 0,
      p90: 14,
      loss: 0,
      hours: 62,
    },
    {
      label: 'Livebox-4F2A',
      connection: 'Wi-Fi',
      solid: 81,
      unstable: 14,
      degraded: 5,
      p90: 38,
      loss: 0.3,
      hours: 140,
    },
    {
      label: 'iPhone de Lucas',
      connection: 'Partage de connexion',
      solid: 52,
      unstable: 35,
      degraded: 13,
      p90: 118,
      loss: 1.1,
      hours: 6,
    },
  ].map((network, index) => ({
    label: network.label,
    connection: network.connection,
    shares: { solid: network.solid, unstable: network.unstable, degraded: network.degraded, offline: 0 },
    solidLabel: `${String(network.solid)}${NBSP}% Solide`,
    sharesLabel: `${String(network.solid)}${NBSP}% Solide, ${String(network.unstable)}${NBSP}% Instable, ${String(network.degraded)}${NBSP}% Dégradée, 0${NBSP}% Hors ligne`,
    latencyP90: ms(network.p90),
    loss: percent(network.loss),
    measured: `${String(network.hours)}${NBSP}h`,
    isCurrent: index === 1,
    isMostReliable: index === 0,
  })),
}

const firstRun: DashboardSummaryDisplay = {
  overview: {
    today: 'Pas encore assez de mesures',
    todayCoverage: '',
    outages: 'Aucune',
    worstMoment: 'Pas encore assez de mesures',
    onNetworkSince: `6${NBSP}s`,
  },
  networks: [],
}

const client = new InMemoryNetworkQualityClient(undefined, connectionScenarios.degraded, windowModeTexts)
client.setThroughputPanel({
  isAllowed: scenario !== 'first',
  note: `Les tests ne partent que sur un clic, vers Cloudflare. Rapide${NBSP}: jusqu'à 4,2${NBSP}Mo. Complet${NBSP}: jusqu'à 155,8${NBSP}Mo. La latence en charge s'affiche sans compter dans l'état.`,
  tests: [
    {
      when: `21/10, 18${NBSP}h${NBSP}02`,
      profile: 'Complet',
      download: `451${NBSP}Mbit/s`,
      upload: `322${NBSP}Mbit/s`,
      loadedLatency: `46${NBSP}ms`,
      data: `138,2${NBSP}Mo`,
    },
    {
      when: `20/10, 09${NBSP}h${NBSP}41`,
      profile: 'Rapide',
      download: `12,5${NBSP}Mbit/s`,
      upload: `1,9${NBSP}Mbit/s`,
      loadedLatency: `335${NBSP}ms`,
      data: `2,7${NBSP}Mo`,
    },
  ],
})
if (scenario === 'first') {
  client.setDashboardSummary(firstRun)
  client.setEvolution({ ...evolution, states: [], isEmpty: true })
} else {
  client.setDashboardSummary(summary)
  client.setEvolution(evolution)
}
client.warnBeforeThroughputTests(`Connexion limitée${NBSP}: ce test consomme jusqu'à 155,8${NBSP}Mo.`)
createApp(App).provide(NETWORK_QUALITY_CLIENT_KEY, client).mount('#app')

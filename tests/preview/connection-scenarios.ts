// Prepared displays shared by the preview pages, worded as the Rust core words them.
import type { ConnectionDisplay } from '../../src/bindings'
import { createConnectionDisplay } from '../fakes/in-memory-network-quality-client'

const DEFINITIONS: Record<string, string> = {
  Latence: "Temps d'un aller-retour vers Internet.",
  Gigue: "Variation de la latence d'un paquet à l'autre.",
  Perte: 'Paquets envoyés qui ne reviennent pas.',
}

export const NBSP = ' '
export const connectionScenarios: Record<string, ConnectionDisplay> = {
  degraded: createConnectionDisplay({
    openDashboardLabel: 'Ouvrir le tableau de bord',
    network: {
      label: 'Wi-Fi (nom masqué)',
      connection: 'Wi-Fi 5 GHz, via un VPN',
      wifiDetails: `−58${NBSP}dBm, canal 36, 866${NBSP}Mbit/s`,
      revealLabel: 'Afficher le nom du réseau',
    },
    state: 'unstable',
    stateLabel: 'Instable',
    cause: "L'opérateur est en cause, le Wi-Fi va bien.",
    action: 'Patienter, ou redémarrer la box si cela dure.',
    path: {
      computer: 'Ce Mac',
      gateway: 'Box',
      internet: 'Internet',
      local: { label: `Wi-Fi, 4${NBSP}ms`, fault: null, state: 'solid', isCut: false },
      internetSegment: { label: `Opérateur, 180${NBSP}ms`, fault: 'en cause', state: 'unstable', isCut: false },
      description: 'Chemin : Ce Mac, Box, Internet. Tronçon Opérateur en cause.',
    },
    detail: `Latence 180${NBSP}ms, seuil 100${NBSP}ms`,
    metrics: [
      {
        name: 'Latence',
        definition: "Temps d'un aller-retour vers Internet.",
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
        definition: "Variation de la latence d'un paquet à l'autre.",
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
        definition: 'Paquets envoyés qui ne reviennent pas.',
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
    network: { label: 'Livebox-4F2A', connection: 'Wi-Fi 5 GHz', wifiDetails: null, revealLabel: null },
    state: 'offline',
    stateLabel: 'Hors ligne',
    cause: 'La box répond, mais Internet ne répond plus.',
    action: "Redémarrer la box ou vérifier le réseau de l'opérateur.",
    path: {
      computer: 'Ce Mac',
      gateway: 'Box',
      internet: 'Internet',
      local: { label: `Wi-Fi, 3${NBSP}ms`, fault: null, state: 'solid', isCut: false },
      internetSegment: { label: 'Opérateur, en attente', fault: 'coupé', state: 'offline', isCut: true },
      description: 'Chemin : Ce Mac, Box, Internet. Tronçon Opérateur coupé.',
    },
    metrics: ['Latence', 'Gigue', 'Perte'].map((name) => ({
      name,
      definition: DEFINITIONS[name] ?? '',
      method: `15${NBSP}s`,
      value: 'en cours',
      gauge: null,
      hint: null,
    })),
  }),
}

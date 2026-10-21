<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'
import type { DashboardSummaryDisplay, EvolutionDisplay, PeriodChoice } from '../bindings'
import { useConnectionDisplay } from '../composables/use-connection-display'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import CurrentMetrics from './CurrentMetrics.vue'
import DashboardState from './DashboardState.vue'
import EvolutionChart from './EvolutionChart.vue'
import NetworksTable from './NetworksTable.vue'
import SegmentedControl from './SegmentedControl.vue'

type ChartMetric = 'latency' | 'jitter' | 'loss'

/** Half the shortest chart point (one minute): a finished minute shows within 30 s. */
const REFRESH_EVERY_MS = 30_000

const METRICS = [
  { value: 'latency', label: 'Latence' },
  { value: 'jitter', label: 'Gigue' },
  { value: 'loss', label: 'Perte' },
] as const satisfies readonly { value: ChartMetric; label: string }[]

const PERIODS = [
  { value: 'hour', label: '1 h', ticks: ['Il y a 1 h', 'Il y a 30 min', 'Maintenant'] },
  { value: 'day', label: '24 h', ticks: ['Il y a 24 h', 'Il y a 12 h', 'Maintenant'] },
  { value: 'week', label: '7 j', ticks: ['Il y a 7 j', 'Il y a 3 j', 'Maintenant'] },
  { value: 'month', label: '30 j', ticks: ['Il y a 30 j', 'Il y a 15 j', 'Maintenant'] },
] as const satisfies readonly { value: PeriodChoice; label: string; ticks: readonly [string, string, string] }[]

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const display = useConnectionDisplay(client)
const updatedAt = ref('')
const summary = ref<DashboardSummaryDisplay | null>(null)
const evolution = ref<EvolutionDisplay | null>(null)
const metric = ref<ChartMetric>('latency')
const period = ref<PeriodChoice>('hour')
let refresh: ReturnType<typeof setInterval> | undefined

const chosenPeriod = computed(() => PERIODS.find((entry) => entry.value === period.value) ?? PERIODS[0])
const chosenMetric = computed(() => METRICS.find((entry) => entry.value === metric.value) ?? METRICS[0])
const chartLabel = computed(
  () => `${chosenMetric.value.label} sur ${chosenPeriod.value.label}, avec les seuils Instable et Dégradée`,
)

watch(display, () => {
  updatedAt.value = new Date().toLocaleTimeString('fr-FR')
})

const loadSummary = async (): Promise<void> => {
  try {
    summary.value = await client.readDashboardSummary(readUtcOffsetSeconds())
  } catch (error) {
    console.error('Reading the dashboard summary failed', error)
  }
}

const loadEvolution = async (): Promise<void> => {
  const requested = period.value
  try {
    const read = await client.readEvolution(requested, readUtcOffsetSeconds())
    // A period picked while this read was in flight wins.
    if (requested === period.value) evolution.value = read
  } catch (error) {
    console.error('Reading the evolution failed', error)
  }
}

watch(period, loadEvolution)

onMounted(async () => {
  refresh = setInterval(() => {
    void loadSummary()
    void loadEvolution()
  }, REFRESH_EVERY_MS)
  await Promise.all([loadSummary(), loadEvolution()])
})

onUnmounted(() => {
  clearInterval(refresh)
})
</script>

<template>
  <div class="dashboard">
    <DashboardState v-if="display" :display="display" :overview="summary?.overview ?? null" :updated-at="updatedAt" />
    <section v-if="display" aria-labelledby="metrics-title">
      <div class="section-head">
        <h2 id="metrics-title">Mesures actuelles</h2>
        <span class="muted">Vers Internet, une mesure par seconde</span>
      </div>
      <CurrentMetrics :metrics="display.metrics" />
    </section>
    <section aria-labelledby="evolution-title">
      <div class="section-head">
        <h2 id="evolution-title">Évolution</h2>
        <div class="controls">
          <SegmentedControl v-model="metric" :options="METRICS" label="Mesure affichée" data-metric-choice />
          <SegmentedControl v-model="period" :options="PERIODS" label="Période" data-period-choice />
        </div>
      </div>
      <EvolutionChart
        v-if="evolution"
        :evolution="evolution"
        :series="evolution[metric]"
        :ticks="chosenPeriod.ticks"
        :label="chartLabel"
      />
    </section>
    <section aria-labelledby="networks-title">
      <div class="section-head">
        <h2 id="networks-title">Tes réseaux</h2>
        <span class="muted">30 derniers jours</span>
      </div>
      <NetworksTable v-if="summary" :networks="summary.networks" />
    </section>
  </div>
</template>

<style scoped>
.dashboard {
  display: grid;
  gap: var(--s7);
  margin-top: var(--s5);
}

.section-head {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s4);
  align-items: baseline;
  margin-bottom: var(--s4);
}

h2 {
  margin: 0;
  font-size: var(--t-lg);
  font-weight: 600;
}

.controls {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s3);
  margin-left: auto;
}

.muted {
  color: var(--muted);
}
</style>

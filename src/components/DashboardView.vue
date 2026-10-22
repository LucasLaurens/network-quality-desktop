<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'
import type { DashboardSummaryDisplay, EvolutionDisplay, PeriodChoice } from '../bindings'
import { useInterfaceTexts } from '../composables/interface-texts'
import { useConnectionDisplay } from '../composables/use-connection-display'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import CurrentMetrics from './CurrentMetrics.vue'
import DashboardState from './DashboardState.vue'
import EvolutionChart from './EvolutionChart.vue'
import NetworksTable from './NetworksTable.vue'
import SegmentedControl from './SegmentedControl.vue'
import TechnicalDetails from './TechnicalDetails.vue'
import ThroughputSection from './ThroughputSection.vue'

type ChartMetric = 'latency' | 'jitter' | 'loss'

/** Half the shortest chart point (one minute): a finished minute shows within 30 s. */
const REFRESH_EVERY_MS = 30_000

const METRICS = [
  { value: 'latency', labelKey: 'ui-metric-latency' },
  { value: 'jitter', labelKey: 'ui-metric-jitter' },
  { value: 'loss', labelKey: 'ui-metric-loss' },
] as const satisfies readonly { value: ChartMetric; labelKey: string }[]

/** Each period with the start and the middle of its time axis; the end is always now. */
const PERIODS = [
  { value: 'hour', labelKey: 'ui-period-hour', tickKeys: ['ui-tick-hour-start', 'ui-tick-hour-middle'] },
  { value: 'day', labelKey: 'ui-period-day', tickKeys: ['ui-tick-day-start', 'ui-tick-day-middle'] },
  { value: 'week', labelKey: 'ui-period-week', tickKeys: ['ui-tick-week-start', 'ui-tick-week-middle'] },
  { value: 'month', labelKey: 'ui-period-month', tickKeys: ['ui-tick-month-start', 'ui-tick-month-middle'] },
] as const satisfies readonly { value: PeriodChoice; labelKey: string; tickKeys: readonly [string, string] }[]

const emit = defineEmits<{ openSettings: [] }>()

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()

const metricOptions = METRICS.map(({ value, labelKey }) => ({ value, label: say(labelKey) }))
const periodOptions = PERIODS.map(({ value, labelKey }) => ({ value, label: say(labelKey) }))

const display = useConnectionDisplay(client)
const updatedAt = ref('')
const summary = ref<DashboardSummaryDisplay | null>(null)
const evolution = ref<EvolutionDisplay | null>(null)
const metric = ref<ChartMetric>('latency')
const period = ref<PeriodChoice>('hour')
let refresh: ReturnType<typeof setInterval> | undefined

const chosenPeriod = computed(() => PERIODS.find((entry) => entry.value === period.value) ?? PERIODS[0])
const chosenMetric = computed(() => METRICS.find((entry) => entry.value === metric.value) ?? METRICS[0])
const ticks = computed(
  () => [say(chosenPeriod.value.tickKeys[0]), say(chosenPeriod.value.tickKeys[1]), say('ui-tick-now')] as const,
)
const chartLabel = computed(() =>
  say('ui-chart-label', { metric: say(chosenMetric.value.labelKey), period: say(chosenPeriod.value.labelKey) }),
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
        <h2 id="metrics-title">{{ say('ui-metrics-title') }}</h2>
        <span class="muted">{{ say('ui-metrics-subtitle') }}</span>
      </div>
      <CurrentMetrics :metrics="display.metrics" />
    </section>
    <section aria-labelledby="evolution-title">
      <div class="section-head">
        <h2 id="evolution-title">{{ say('ui-evolution-title') }}</h2>
        <div class="controls">
          <SegmentedControl
            v-model="metric"
            :options="metricOptions"
            :label="say('ui-evolution-metric')"
            data-metric-choice
          />
          <SegmentedControl
            v-model="period"
            :options="periodOptions"
            :label="say('ui-evolution-period')"
            data-period-choice
          />
        </div>
      </div>
      <EvolutionChart
        v-if="evolution"
        :evolution="evolution"
        :series="evolution[metric]"
        :ticks="ticks"
        :label="chartLabel"
      />
    </section>
    <section aria-labelledby="networks-title">
      <div class="section-head">
        <h2 id="networks-title">{{ say('ui-networks-title') }}</h2>
        <span class="muted">{{ say('ui-last-month') }}</span>
      </div>
      <NetworksTable v-if="summary" :networks="summary.networks" />
    </section>
    <section aria-labelledby="throughput-title">
      <div class="section-head">
        <h2 id="throughput-title">{{ say('ui-throughput-title') }}</h2>
      </div>
      <ThroughputSection @open-settings="emit('openSettings')" />
    </section>
    <TechnicalDetails />
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

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useConnectionDisplay } from '../composables/use-connection-display'
import { useNetworkQualityClient } from '../services/network-quality-client'
import ConnectionPath from './ConnectionPath.vue'
import MetricGauge from './MetricGauge.vue'
import HelpDetails from './HelpDetails.vue'
import PopoverThroughput from './PopoverThroughput.vue'
import StateShape from './StateShape.vue'

const client = useNetworkQualityClient()

const display = useConnectionDisplay(client)

const closeOnEscape = (event: KeyboardEvent): void => {
  if (event.key === 'Escape') void client.closePopover()
}

onMounted(() => {
  window.addEventListener('keydown', closeOnEscape)
})

onUnmounted(() => {
  window.removeEventListener('keydown', closeOnEscape)
})

const revealNetworkName = async (): Promise<void> => {
  await client.revealNetworkName()
}

const openDashboard = async (): Promise<void> => {
  await client.openDashboard()
}
</script>

<template>
  <main v-if="display" class="popover" :data-state="display.state" aria-labelledby="state-label">
    <header class="head">
      <StateShape :state="display.state" />
      <h1 id="state-label">{{ display.stateLabel }}</h1>
      <p v-if="display.network" class="where" data-network>
        <span>{{ display.network.label }}</span>
        <span class="connection">{{ display.network.connection }}</span>
        <span v-if="display.network.wifiDetails" class="connection num" data-wifi-details>
          {{ display.network.wifiDetails }}
        </span>
        <button
          v-if="display.network.revealLabel"
          type="button"
          class="reveal"
          data-reveal-name
          @click="revealNetworkName"
        >
          {{ display.network.revealLabel }}
        </button>
      </p>
    </header>
    <p class="cause" aria-live="polite">{{ display.cause }}</p>
    <p v-if="display.action" class="action">{{ display.action }}</p>
    <HelpDetails v-if="display.help" :help="display.help" />
    <p v-if="display.detail" class="detail num">{{ display.detail }}</p>
    <ConnectionPath v-if="display.path" :path="display.path" />

    <ul class="metrics">
      <li v-for="metric in display.metrics" :key="metric.name" class="metric" data-metric>
        <span class="name">
          {{ metric.name }} <span class="method">{{ metric.method }}</span>
        </span>
        <span class="value num">{{ metric.value }}</span>
        <MetricGauge :gauge="metric.gauge" class="gauge" />
        <span v-if="metric.hint" class="hint">{{ metric.hint }}</span>
      </li>
    </ul>

    <PopoverThroughput />
    <button type="button" class="primary" data-primary @click="openDashboard">{{ display.openDashboardLabel }}</button>
  </main>
</template>

<style scoped>
.popover {
  min-height: 100vh;
  padding: var(--s4);
  background: var(--surface);
}

.head {
  display: flex;
  align-items: center;
  gap: var(--s2);
}

.where {
  display: grid;
  margin: 0 0 0 auto;
  color: var(--muted);
  font-size: var(--t-sm);
  text-align: right;
}

.where .connection {
  font-size: var(--t-xs);
}

.reveal {
  margin-top: 2px;
  padding: 0;
  border: 0;
  background: none;
  color: var(--ink);
  font: inherit;
  font-size: var(--t-xs);
  text-decoration: underline;
  cursor: pointer;
}

h1 {
  margin: 0;
  font-size: var(--t-xl);
  font-weight: 600;
}

.cause {
  margin: var(--s2) 0 0;
}

.action {
  margin: var(--s1) 0 0;
  font-weight: 500;
}

.detail {
  margin: var(--s1) 0 0;
  color: var(--muted);
  font-size: var(--t-sm);
}

.metrics {
  display: grid;
  gap: var(--s3);
  margin: var(--s4) 0 0;
  padding: var(--s4) 0 0;
  border-top: 1px solid var(--line);
  list-style: none;
}

.metric {
  display: grid;
  grid-template-columns: 1fr auto;
  row-gap: var(--s1);
  align-items: baseline;
}

.method {
  color: var(--muted);
  font-size: var(--t-xs);
}

.value {
  font-weight: 600;
}

.gauge,
.hint {
  grid-column: 1 / -1;
}

.hint {
  color: var(--muted);
  font-size: var(--t-xs);
}

.num {
  font-variant-numeric: tabular-nums;
}

.primary {
  width: 100%;
  min-height: 32px;
  margin-top: var(--s4);
  padding: 0 var(--s3);
  border: 1px solid var(--ink);
  border-radius: var(--radius);
  background: var(--ink);
  color: var(--surface);
  font: inherit;
  font-weight: 500;
  cursor: pointer;
}

.primary:hover {
  background: oklch(from var(--ink) calc(l + 0.06) c h);
}
</style>

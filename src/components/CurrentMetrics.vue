<script setup lang="ts">
import type { MetricDisplay } from '../bindings'
import MetricGauge from './MetricGauge.vue'

const { metrics } = defineProps<{ metrics: MetricDisplay[] }>()
</script>

<template>
  <ul class="metrics">
    <li v-for="metric in metrics" :key="metric.name" class="metric" data-metric>
      <span class="name">{{ metric.name }}</span>
      <span class="definition">{{ metric.definition }}</span>
      <span class="value num">{{ metric.value }}</span>
      <MetricGauge :gauge="metric.gauge" class="gauge" />
      <span v-if="metric.gauge" class="small" data-thresholds>{{ metric.gauge.label }}</span>
      <span class="small">{{ metric.method }}</span>
      <span v-if="metric.hint" class="small hint" data-hint>{{ metric.hint }}</span>
    </li>
  </ul>
</template>

<style scoped>
.metrics {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  margin: 0;
  padding: 0;
  list-style: none;
}

.metric {
  display: grid;
  gap: var(--s1);
  align-content: start;
  padding-right: var(--s5);
}

.metric + .metric {
  padding-left: var(--s5);
  border-left: 1px solid var(--line);
}

.name {
  font-weight: 600;
}

.definition {
  min-height: calc(2 * 1.45em);
  color: var(--muted);
  font-size: var(--t-xs);
}

.value {
  margin-top: var(--s2);
  font-size: var(--t-xl);
  font-weight: 600;
}

.gauge {
  margin: var(--s2) 0 var(--s1);
}

.small {
  color: var(--muted);
  font-size: var(--t-xs);
}

.hint {
  color: var(--ink);
}

.num {
  font-variant-numeric: tabular-nums;
}

@media (max-width: 640px) {
  .metrics {
    grid-template-columns: 1fr;
    gap: var(--s5);
  }

  .metric + .metric {
    padding-left: 0;
    border-left: 0;
  }
}
</style>

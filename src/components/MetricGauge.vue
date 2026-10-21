<script setup lang="ts">
import type { Gauge } from '../bindings'

const { gauge } = defineProps<{ gauge: Gauge | null }>()

const toPercent = (fraction: number | null): string => `${String(Math.round((fraction ?? 0) * 1000) / 10)}%`
</script>

<!-- Neutral zones (Solide, Instable, Dégradée) and a marker for the value; positions come from the core. -->
<template>
  <div class="gauge" aria-hidden="true">
    <span class="zone" :style="{ width: toPercent(gauge?.unstableFrom ?? 0.25) }" />
    <span class="zone" :style="{ width: toPercent((gauge?.degradedAbove ?? 0.75) - (gauge?.unstableFrom ?? 0.25)) }" />
    <span class="zone rest" />
    <i v-if="gauge" class="marker" data-marker :style="{ left: toPercent(gauge.position) }" />
  </div>
</template>

<style scoped>
.gauge {
  position: relative;
  display: flex;
  gap: 2px;
  height: 6px;
}

.zone {
  height: 100%;
  border-radius: 2px;
  background: var(--zone);
}

.rest {
  flex: 1;
}

.marker {
  position: absolute;
  top: -3px;
  width: 3px;
  height: 12px;
  margin-left: -1.5px;
  border-radius: 2px;
  background: var(--ink);
}
</style>

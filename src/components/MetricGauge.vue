<script setup lang="ts">
import type { Gauge } from '../bindings'

const { gauge } = defineProps<{ gauge: Gauge | null }>()

// Rust f64 is exported as `number | null`, since serde writes NaN as null.
const toPercent = (fraction: number | null): string => `${String(Math.round((fraction ?? 0) * 1000) / 10)}%`
</script>

<!--
  Neutral zones (Solide, Instable, Dégradée) and a marker for the value; positions come from the core.
  An unmeasured metric shows an empty track.
-->
<template>
  <div v-if="gauge" class="gauge" role="img" :aria-label="gauge.label">
    <span class="zone" data-zone :style="{ width: toPercent(gauge.unstableFrom) }" />
    <span
      class="zone"
      data-zone
      :style="{ width: toPercent((gauge.degradedAbove ?? 0) - (gauge.unstableFrom ?? 0)) }"
    />
    <span class="zone rest" data-zone />
    <i class="marker" data-marker :style="{ left: toPercent(gauge.position) }" />
  </div>
  <div v-else class="gauge" aria-hidden="true">
    <span class="zone rest" />
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

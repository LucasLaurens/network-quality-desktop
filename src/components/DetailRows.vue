<script setup lang="ts">
import type { DetailRow } from '../bindings'

const { rows } = defineProps<{ rows: DetailRow[] }>()
</script>

<template>
  <dl>
    <div v-for="row in rows" :key="row.label" class="row">
      <dt>{{ row.label }}</dt>
      <dd v-if="row.spokenValue === row.value">{{ row.value }}</dd>
      <!-- Arrows mean nothing read aloud: a screen reader gets the words. -->
      <dd v-else>
        <span aria-hidden="true">{{ row.value }}</span
        ><span class="visually-hidden">{{ row.spokenValue }}</span>
      </dd>
    </div>
  </dl>
</template>

<style scoped>
dl {
  margin: 0;
}

.row {
  display: flex;
  gap: var(--s3);
  align-items: baseline;
  justify-content: space-between;
  padding: var(--s1) 0;
  border-bottom: 1px solid var(--line);
}

dt {
  color: var(--muted);
}

dd {
  margin: 0;
  font-variant-numeric: tabular-nums;
  text-align: right;
  white-space: nowrap;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>

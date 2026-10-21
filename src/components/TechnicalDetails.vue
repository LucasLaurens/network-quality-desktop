<script setup lang="ts">
import { computed, inject, onUnmounted, ref } from 'vue'
import type { LineDetails, TechnicalDetails } from '../bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'

/** The traffic is averaged over 5 s in the core: reading it faster shows nothing new. */
const REFRESH_EVERY_MS = 5000

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const details = ref<TechnicalDetails | null>(null)
const line = ref<LineDetails | null>(null)
let refresh: ReturnType<typeof setInterval> | undefined

const sections = computed(() => [...(details.value?.sections ?? []), ...(line.value ? [line.value.section] : [])])

const loadDetails = async (): Promise<void> => {
  try {
    details.value = await client.readTechnicalDetails()
  } catch (error) {
    console.error('Reading the technical details failed', error)
  }
}

const loadLine = async (): Promise<void> => {
  try {
    line.value = await client.readLineDetails()
  } catch (error) {
    console.error('Reading the line failed', error)
  }
}

const stopRefreshing = (): void => {
  clearInterval(refresh)
  refresh = undefined
}

// Nothing is read while folded: the box is only asked when the user looks.
const onToggle = (event: Event): void => {
  stopRefreshing()
  if (!(event.target as HTMLDetailsElement).open) return
  void loadDetails()
  void loadLine()
  refresh = setInterval(() => void loadDetails(), REFRESH_EVERY_MS)
}

onUnmounted(stopRefreshing)
</script>

<template>
  <details class="details" data-details @toggle="onToggle">
    <summary>Détails techniques</summary>
    <div class="sections">
      <section v-for="section in sections" :key="section.title" data-detail-section>
        <h3>{{ section.title }}</h3>
        <dl>
          <div v-for="row in section.rows" :key="row.label" class="row">
            <dt>{{ row.label }}</dt>
            <dd>{{ row.value }}</dd>
          </div>
        </dl>
      </section>
    </div>
    <p v-if="line" class="small" data-line-note>{{ line.note }}</p>
    <p v-else-if="details" class="small" role="status">Interrogation de la box…</p>
  </details>
</template>

<style scoped>
summary {
  font-weight: 600;
  cursor: pointer;
}

.sections {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: var(--s5);
  margin-top: var(--s4);
}

h3 {
  margin: 0 0 var(--s2);
  color: var(--muted);
  font-size: var(--t-sm);
  font-weight: 600;
}

dl {
  margin: 0;
}

.row {
  display: flex;
  gap: var(--s3);
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

.small {
  margin: var(--s4) 0 0;
  color: var(--muted);
  font-size: var(--t-xs);
}
</style>

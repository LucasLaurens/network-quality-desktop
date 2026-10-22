<script setup lang="ts">
import { useInterfaceTexts } from '../composables/interface-texts'
import { computed, inject, onUnmounted, ref } from 'vue'
import type { LineDetails, TechnicalDetails } from '../bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()

const details = ref<TechnicalDetails | null>(null)
const line = ref<LineDetails | null>(null)
const lineRefusal = ref('')
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
  lineRefusal.value = ''
  try {
    line.value = await client.readLineDetails()
  } catch (error) {
    lineRefusal.value = error instanceof Error ? error.message : String(error)
  }
}

const stopRefreshing = (): void => {
  clearInterval(refresh)
  refresh = undefined
}

// Nothing is read while folded: the box is only asked when the user looks.
const onToggle = async (event: Event): Promise<void> => {
  stopRefreshing()
  const element = event.target as HTMLDetailsElement
  const isOpen = (): boolean => element.open
  if (!isOpen()) return
  void loadLine()
  await loadDetails()
  // Folded again while the details were read: nothing to refresh.
  if (!isOpen() || !details.value) return
  refresh = setInterval(() => void loadDetails(), details.value.refreshSeconds * 1000)
}

onUnmounted(stopRefreshing)
</script>

<template>
  <details class="details" data-details @toggle="onToggle">
    <summary>{{ say('ui-details-title') }}</summary>
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
    <p v-else-if="lineRefusal" class="small" role="alert" data-line-refused>{{ lineRefusal }}</p>
    <p v-else-if="details" class="small" role="status">{{ say('ui-details-asking-box') }}</p>
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

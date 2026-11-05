<script setup lang="ts">
import { useInterfaceTexts } from '../composables/interface-texts'
import { computed, onUnmounted, ref } from 'vue'
import type { LineDetails, TechnicalDetails } from '../bindings'
import { readErrorMessage, useNetworkQualityClient } from '../services/network-quality-client'
import DetailRows from './DetailRows.vue'

const client = useNetworkQualityClient()
const say = useInterfaceTexts()

const details = ref<TechnicalDetails | null>(null)
const line = ref<LineDetails | null>(null)
const lineRefusal = ref('')
let refresh: ReturnType<typeof setInterval> | undefined

const sections = computed(() => details.value?.sections ?? [])

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
    lineRefusal.value = readErrorMessage(error)
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
        <DetailRows :rows="section.rows" />
      </section>
      <!-- The box answers after a few seconds: the line is announced once it arrives. -->
      <div v-if="details" class="line" aria-live="polite" data-line-section>
        <section v-if="line" data-detail-section>
          <h3>{{ line.section.title }}</h3>
          <DetailRows :rows="line.section.rows" aria-describedby="line-note" />
          <p id="line-note" class="small" data-line-note>{{ line.note }}</p>
        </section>
        <p v-else-if="lineRefusal" class="small" data-line-refused>{{ lineRefusal }}</p>
        <p v-else class="small">{{ say('ui-details-asking-box') }}</p>
      </div>
    </div>
  </details>
</template>

<style scoped>
summary {
  font-weight: 600;
  cursor: pointer;
}

.sections {
  display: grid;
  /* Two columns on the dashboard, one in a narrow window: never a lone section on a third row. */
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: var(--s5) var(--s6);
  margin-top: var(--s4);
}

h3 {
  margin: 0 0 var(--s2);
  color: var(--muted);
  font-size: var(--t-sm);
  font-weight: 600;
}

.small {
  margin: var(--s3) 0 0;
  color: var(--muted);
  font-size: var(--t-xs);
}

.line > .small:only-child {
  margin: 0;
}
</style>

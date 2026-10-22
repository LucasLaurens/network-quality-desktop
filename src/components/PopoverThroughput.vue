<script setup lang="ts">
import { computed, inject, ref } from 'vue'
import type { ThroughputPanel } from '../bindings'
import { useLoadOnFocus } from '../composables/use-load-on-focus'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import ThroughputControls from './ThroughputControls.vue'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const panel = ref<ThroughputPanel | null>(null)
const lastTest = computed(() => panel.value?.tests[0] ?? null)

const loadPanel = async (): Promise<void> => {
  try {
    panel.value = await client.readThroughputPanel(readUtcOffsetSeconds())
  } catch (error) {
    console.error('Reading the throughput tests failed', error)
  }
}

const openSettings = async (): Promise<void> => {
  await client.openSettings()
}

useLoadOnFocus(loadPanel)
</script>

<template>
  <section v-if="panel" class="throughput" aria-label="Débit">
    <p v-if="!panel.isAllowed" class="small">
      Tests de débit désactivés&nbsp;:
      <button type="button" class="link" data-open-settings @click="openSettings">Activer dans Réglages</button>
    </p>
    <template v-else>
      <ThroughputControls @done="loadPanel" />
      <p v-if="lastTest" class="small num" data-last-test>
        Dernier test, {{ lastTest.when }}&nbsp;: réception {{ lastTest.download }}, envoi {{ lastTest.upload }}
      </p>
    </template>
  </section>
</template>

<style scoped>
.throughput {
  display: grid;
  gap: var(--s2);
  margin-top: var(--s4);
  padding-top: var(--s4);
  border-top: 1px solid var(--line);
}

.link {
  padding: 0;
  border: 0;
  background: none;
  color: var(--ink);
  font: inherit;
  font-size: var(--t-sm);
  text-align: left;
  text-decoration: underline;
  cursor: pointer;
}

.small {
  margin: 0;
  color: var(--muted);
  font-size: var(--t-xs);
}

.num {
  font-variant-numeric: tabular-nums;
}
</style>

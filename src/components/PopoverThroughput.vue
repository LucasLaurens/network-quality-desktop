<script setup lang="ts">
import { computed, inject, onMounted, ref } from 'vue'
import type { ProfileChoice, ThroughputPanel } from '../bindings'
import { useThroughputTest } from '../composables/use-throughput-test'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'

const PROFILES = [
  { value: 'quick', label: 'Test rapide' },
  { value: 'full', label: 'Test complet' },
] as const satisfies readonly { value: ProfileChoice; label: string }[]

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

const { running, asked, refusal, run, cancel } = useThroughputTest(client, loadPanel)

const openSettings = async (): Promise<void> => {
  await client.openSettings()
}

onMounted(loadPanel)
</script>

<template>
  <section v-if="panel" class="throughput" aria-label="Débit">
    <p v-if="!panel.isAllowed" class="small">
      Tests de débit désactivés&nbsp;:
      <button type="button" class="link" data-open-settings @click="openSettings">Activer dans Réglages</button>
    </p>
    <template v-else>
      <div class="actions">
        <button
          v-for="profile in PROFILES"
          :key="profile.value"
          type="button"
          class="secondary"
          :disabled="running !== null"
          :aria-busy="running === profile.value"
          :data-run="profile.value"
          @click="run(profile.value, false)"
        >
          {{ running === profile.value ? 'Test en cours…' : profile.label }}
        </button>
      </div>
      <div v-if="asked" class="warning" role="alertdialog" aria-labelledby="popover-warning" data-warning>
        <p id="popover-warning">{{ asked.warning }}</p>
        <div class="actions">
          <button type="button" class="secondary" data-confirm @click="run(asked.profile, true)">Lancer le test</button>
          <button type="button" class="link" data-cancel @click="cancel">Annuler</button>
        </div>
      </div>
      <p v-if="refusal" class="small" role="alert" data-refused>{{ refusal }}</p>
      <p v-if="lastTest" class="small num" data-last-test>
        Dernier test, {{ lastTest.when }}&nbsp;: ↓ {{ lastTest.download }}, ↑ {{ lastTest.upload }}
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

.actions {
  display: flex;
  gap: var(--s2);
  align-items: center;
}

.secondary {
  flex: 1;
  min-height: 28px;
  padding: 0 var(--s3);
  border: 1px solid var(--line);
  border-radius: var(--radius-nested);
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-weight: 500;
  cursor: pointer;
}

.secondary:disabled {
  color: var(--muted);
  cursor: default;
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

.warning {
  display: grid;
  gap: var(--s2);
}

.warning p,
.small {
  margin: 0;
}

.small {
  color: var(--muted);
  font-size: var(--t-xs);
}

.num {
  font-variant-numeric: tabular-nums;
}
</style>

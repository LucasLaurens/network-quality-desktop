<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { ProfileChoice, ThroughputPanel } from '../bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'

const PROFILES = [
  { value: 'quick', label: 'Test rapide' },
  { value: 'full', label: 'Test complet' },
] as const satisfies readonly { value: ProfileChoice; label: string }[]

const emit = defineEmits<{ openSettings: [] }>()

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const panel = ref<ThroughputPanel | null>(null)
const running = ref<ProfileChoice | null>(null)
/** The test waiting for the user's go-ahead, where data counts. */
const asked = ref<ProfileChoice | null>(null)
const refusal = ref('')

const readWarning = (profile: ProfileChoice): string | null =>
  profile === 'quick' ? (panel.value?.quickWarning ?? null) : (panel.value?.fullWarning ?? null)

const run = async (profile: ProfileChoice): Promise<void> => {
  if (!panel.value || running.value) return
  asked.value = null
  refusal.value = ''
  running.value = profile
  try {
    const row = await client.runThroughputTest(profile, readUtcOffsetSeconds())
    panel.value.tests = [row, ...panel.value.tests]
  } catch (error) {
    refusal.value = error instanceof Error ? error.message : String(error)
  } finally {
    running.value = null
  }
}

const choose = async (profile: ProfileChoice): Promise<void> => {
  if (readWarning(profile) === null) {
    await run(profile)
    return
  }
  asked.value = profile
}

onMounted(async () => {
  try {
    panel.value = await client.readThroughputPanel(readUtcOffsetSeconds())
  } catch (error) {
    console.error('Reading the throughput tests failed', error)
  }
})
</script>

<template>
  <div v-if="panel" class="throughput">
    <p v-if="!panel.isAllowed" class="quiet">
      <span>Tests de débit désactivés&nbsp;: aucune donnée n'est consommée.</span>
      <button type="button" class="link" data-open-settings @click="emit('openSettings')">Activer dans Réglages</button>
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
          @click="choose(profile.value)"
        >
          {{ running === profile.value ? 'Test en cours…' : profile.label }}
        </button>
      </div>
      <div v-if="asked" class="warning" role="alertdialog" aria-labelledby="throughput-warning" data-warning>
        <p id="throughput-warning">{{ readWarning(asked) }}</p>
        <div class="actions">
          <button type="button" class="secondary" data-confirm @click="run(asked)">Lancer le test</button>
          <button type="button" class="link" data-cancel @click="asked = null">Annuler</button>
        </div>
      </div>
      <p v-if="refusal" class="refused" role="alert" data-refused>{{ refusal }}</p>
      <div v-if="panel.tests.length > 0" class="scroll">
        <table>
          <thead>
            <tr>
              <th>Quand</th>
              <th>Test</th>
              <th class="right">Réception</th>
              <th class="right">Envoi</th>
              <th class="right">Latence en charge</th>
              <th class="right">Données</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(test, index) in panel.tests" :key="test.when + String(index)" data-test-row>
              <td class="num">{{ test.when }}</td>
              <td>{{ test.profile }}</td>
              <td class="right num">{{ test.download }}</td>
              <td class="right num">{{ test.upload }}</td>
              <td class="right num">{{ test.loadedLatency }}</td>
              <td class="right num">{{ test.data }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="small">
        Les tests ne partent que sur un clic, vers Cloudflare. Rapide&nbsp;: environ 5&nbsp;Mo. Complet&nbsp;: jusqu'à
        160&nbsp;Mo. La latence en charge s'affiche sans compter dans l'état.
      </p>
    </template>
  </div>
</template>

<style scoped>
.throughput {
  display: grid;
  gap: var(--s3);
}

.quiet {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s4);
  align-items: baseline;
  margin: 0;
  color: var(--muted);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s3);
  align-items: center;
}

.secondary {
  min-height: 32px;
  padding: 0 var(--s4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
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
  text-decoration: underline;
  cursor: pointer;
}

.warning {
  display: grid;
  gap: var(--s2);
  max-width: 560px;
  padding: var(--s3) var(--s4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface);
}

.warning p,
.refused {
  margin: 0;
}

.scroll {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th,
td {
  padding: var(--s2) var(--s3) var(--s2) 0;
  border-bottom: 1px solid var(--line);
  text-align: left;
  white-space: nowrap;
}

th {
  color: var(--muted);
  font-size: var(--t-xs);
  font-weight: 500;
}

.right {
  padding-left: var(--s3);
  text-align: right;
}

th:last-child,
td:last-child {
  padding-right: 0;
}

.num {
  font-variant-numeric: tabular-nums;
}

.small {
  margin: 0;
  color: var(--muted);
  font-size: var(--t-xs);
}
</style>

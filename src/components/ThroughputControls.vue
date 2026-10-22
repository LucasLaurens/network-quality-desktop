<script setup lang="ts">
import { useInterfaceTexts } from '../composables/interface-texts'
import { inject, ref, watch } from 'vue'
import { REQUESTED_TEST_KEY } from '../composables/dashboard-requests'
import { THROUGHPUT_PROFILES, useThroughputTest } from '../composables/use-throughput-test'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'

const emit = defineEmits<{ done: [] }>()

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()
// Only the dashboard provides it: the tray menu runs its tests there.
const requested = inject(REQUESTED_TEST_KEY, ref(null))

const { running, asked, refusal, run, cancel } = useThroughputTest(client, () => {
  emit('done')
  return Promise.resolve()
})

watch(
  requested,
  (profile) => {
    if (!profile) return
    requested.value = null
    void run(profile, false)
  },
  { immediate: true },
)
</script>

<template>
  <div class="controls">
    <div class="actions">
      <button
        v-for="profile in THROUGHPUT_PROFILES"
        :key="profile.value"
        type="button"
        class="secondary"
        :disabled="running !== null"
        :aria-busy="running === profile.value"
        :data-run="profile.value"
        @click="run(profile.value, false)"
      >
        {{ running === profile.value ? say('ui-test-running') : say(profile.labelKey) }}
      </button>
    </div>
    <div v-if="asked" class="warning" role="alertdialog" aria-labelledby="throughput-warning" data-warning>
      <p id="throughput-warning">{{ asked.warning }}</p>
      <div class="actions">
        <button type="button" class="secondary" data-confirm @click="run(asked.profile, true)">
          {{ say('ui-test-confirm') }}
        </button>
        <button type="button" class="link" data-cancel @click="cancel">{{ say('ui-cancel') }}</button>
      </div>
    </div>
    <p v-if="refusal" class="refused" role="alert" data-refused>{{ refusal }}</p>
  </div>
</template>

<style scoped>
.controls {
  display: grid;
  gap: var(--s3);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s2);
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
</style>

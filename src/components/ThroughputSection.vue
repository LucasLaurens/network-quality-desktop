<script setup lang="ts">
import { useInterfaceTexts } from '../composables/interface-texts'
import { inject, ref } from 'vue'
import type { ThroughputPanel } from '../bindings'
import { useLoadOnFocus } from '../composables/use-load-on-focus'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import ThroughputControls from './ThroughputControls.vue'

const emit = defineEmits<{ openSettings: [] }>()

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()

const panel = ref<ThroughputPanel | null>(null)

const loadPanel = async (): Promise<void> => {
  try {
    panel.value = await client.readThroughputPanel(readUtcOffsetSeconds())
  } catch (error) {
    console.error('Reading the throughput tests failed', error)
  }
}

useLoadOnFocus(loadPanel)
</script>

<template>
  <div v-if="panel" class="throughput">
    <p v-if="!panel.isAllowed" class="quiet">
      <span>{{ say('ui-throughput-off') }}</span>
      <button type="button" class="link" data-open-settings @click="emit('openSettings')">
        {{ say('ui-throughput-enable') }}
      </button>
    </p>
    <template v-else>
      <ThroughputControls @done="loadPanel" />
      <div v-if="panel.tests.length > 0" class="scroll">
        <table>
          <thead>
            <tr>
              <th>{{ say('ui-throughput-when') }}</th>
              <th>{{ say('ui-throughput-test') }}</th>
              <th class="right">{{ say('ui-throughput-download') }}</th>
              <th class="right">{{ say('ui-throughput-upload') }}</th>
              <th class="right">{{ say('ui-throughput-loaded-latency') }}</th>
              <th class="right">{{ say('ui-throughput-data') }}</th>
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
      <p class="small" data-note>{{ panel.note }}</p>
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

.link {
  padding: 0;
  border: 0;
  background: none;
  color: var(--ink);
  font: inherit;
  text-decoration: underline;
  cursor: pointer;
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

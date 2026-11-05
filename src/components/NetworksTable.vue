<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useInterfaceTexts } from '../composables/interface-texts'
import { useNetworkQualityClient } from '../services/network-quality-client'
import type { NetworkRowDisplay, StateShares } from '../bindings'

const { networks } = defineProps<{ networks: NetworkRowDisplay[] }>()
const emit = defineEmits<{ placeNamed: [] }>()
const say = useInterfaceTexts()
const client = useNetworkQualityClient()

const STATES = ['solid', 'unstable', 'degraded', 'offline'] as const satisfies readonly (keyof StateShares)[]

/** The network whose place is being typed; one at a time. */
const editedId = ref('')
const typedPlace = ref('')
const refusal = ref('')
const isSaving = ref(false)
const field = ref<HTMLInputElement[]>([])

const startEditing = async (network: NetworkRowDisplay): Promise<void> => {
  editedId.value = network.networkId
  typedPlace.value = network.place
  refusal.value = ''
  await nextTick()
  field.value[0]?.focus()
}

const stopEditing = (): void => {
  editedId.value = ''
  refusal.value = ''
}

const savePlace = async (): Promise<void> => {
  if (isSaving.value) return
  isSaving.value = true
  try {
    await client.savePlace(editedId.value, typedPlace.value)
    stopEditing()
    emit('placeNamed')
  } catch (error) {
    refusal.value = error instanceof Error ? error.message : String(error)
  } finally {
    isSaving.value = false
  }
}
</script>

<template>
  <p v-if="networks.length === 0" class="empty" data-networks-empty>
    {{ say('ui-networks-empty') }}
  </p>
  <div v-else class="scroll">
    <table>
      <thead>
        <tr>
          <th>{{ say('ui-networks-network') }}</th>
          <th>{{ say('ui-networks-shares') }}</th>
          <th class="right">{{ say('ui-networks-latency') }}</th>
          <th class="right">{{ say('ui-metric-loss') }}</th>
          <th class="right">{{ say('ui-networks-measured') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="network in networks" :key="network.networkId" data-network-row>
          <td>
            <form
              v-if="editedId === network.networkId"
              class="place-form"
              data-place-form
              @submit.prevent="savePlace"
              @keydown.esc="stopEditing"
            >
              <input
                ref="field"
                v-model="typedPlace"
                type="text"
                autocomplete="off"
                :placeholder="say('ui-place-placeholder')"
                :aria-label="say('ui-place-field', { network: network.label })"
                :aria-invalid="refusal !== ''"
                :aria-describedby="refusal === '' ? undefined : 'place-refusal'"
                data-place-input
              />
              <span class="place-actions">
                <button type="submit" class="secondary" :disabled="isSaving" data-place-save>
                  {{ say('ui-place-save') }}
                </button>
                <button type="button" class="link" data-place-cancel @click="stopEditing">
                  {{ say('ui-cancel') }}
                </button>
              </span>
              <span v-if="refusal" id="place-refusal" class="refusal" role="alert" data-place-refusal>{{
                refusal
              }}</span>
            </form>
            <template v-else>
              <button
                v-if="network.place"
                type="button"
                class="place"
                :aria-label="say('ui-place-rename', { network: network.label })"
                data-place
                @click="startEditing(network)"
              >
                {{ network.place }}</button
              ><template v-else>{{ network.label }}</template
              ><span v-if="network.isCurrent" class="tag" data-current>{{ say('ui-networks-current') }}</span>
              <div class="small">
                <template v-if="network.place">{{ network.label }} · </template>{{ network.connection
                }}<template v-if="network.isMostReliable">
                  · <strong data-most-reliable>{{ say('ui-networks-most-reliable') }}</strong></template
                ><template v-if="!network.place">
                  ·
                  <button type="button" class="link small" data-place-name @click="startEditing(network)">
                    {{ say('ui-place-name') }}
                  </button></template
                >
              </div>
            </template>
          </td>
          <td>
            <div class="shares">
              <div class="stack" role="img" :aria-label="network.sharesLabel">
                <template v-for="state in STATES" :key="state">
                  <span
                    v-if="(network.shares[state] ?? 0) > 0"
                    :style="{ width: `${String(network.shares[state] ?? 0)}%`, background: `var(--${state})` }"
                  />
                </template>
              </div>
              <span class="num small solid">{{ network.solidLabel }}</span>
            </div>
          </td>
          <td class="right num">{{ network.latencyP90 }}</td>
          <td class="right num">{{ network.loss }}</td>
          <td class="right num" data-measured>{{ network.measured }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.scroll {
  overflow-x: auto;
}

th,
td {
  vertical-align: top;
}

.right {
  padding-left: var(--s3);
  text-align: right;
}

th:last-child,
td:last-child {
  padding-right: 0;
}

.tag {
  margin-left: var(--s2);
  color: var(--muted);
  font-size: var(--t-xs);
}

.small {
  color: var(--muted);
  font-size: var(--t-xs);
}

.link.small {
  color: var(--muted);
}

/* The place reads as the row's name; underlined on hover only, it stays editable. */
.place {
  padding: 0;
  border: 0;
  background: none;
  color: var(--ink);
  font: inherit;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}

.place:hover {
  text-decoration: underline;
}

.place-form {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s2);
  align-items: center;
}

.place-form input {
  width: 14em;
  max-width: 100%;
  min-height: 28px;
  padding: 0 var(--s2);
  border: 1px solid var(--line);
  border-radius: var(--radius-nested);
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}

.place-form input[aria-invalid='true'] {
  border-color: var(--degraded);
}

.place-actions {
  display: flex;
  gap: var(--s3);
  align-items: center;
}

.secondary {
  min-height: 28px;
  padding: 0 var(--s3);
  border: 1px solid var(--line);
  border-radius: var(--radius-nested);
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  cursor: pointer;
}

.secondary:disabled {
  color: var(--muted);
  cursor: default;
}

.refusal {
  flex-basis: 100%;
  color: var(--degraded);
  font-size: var(--t-xs);
}

.shares {
  display: flex;
  gap: 10px;
  align-items: center;
}

.stack {
  display: flex;
  gap: 2px;
  width: 140px;
  height: 6px;
}

.stack span {
  height: 100%;
  border-radius: 2px;
}

.solid {
  color: var(--ink);
}

.num {
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 0;
  padding: var(--s5) 0;
  color: var(--muted);
}

@media (max-width: 640px) {
  .stack {
    width: 80px;
  }
}
</style>

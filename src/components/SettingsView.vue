<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { AppearanceChoice, ExportPeriodChoice, SettingsForm } from '../bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import { useLoadOnFocus } from '../composables/use-load-on-focus'
import { useInterfaceTexts } from '../composables/interface-texts'

const PUBLIC_SERVERS = 'public'
const OWN_SERVER = 'own'
type ServersChoice = typeof PUBLIC_SERVERS | typeof OWN_SERVER

const APPEARANCES = [
  { value: 'system', labelKey: 'ui-appearance-system' },
  { value: 'light', labelKey: 'ui-appearance-light' },
  { value: 'dark', labelKey: 'ui-appearance-dark' },
] as const satisfies readonly { value: AppearanceChoice; labelKey: string }[]

const EXPORT_PERIODS = [
  { value: 'lastDay', labelKey: 'ui-export-last-day' },
  { value: 'lastWeek', labelKey: 'ui-export-last-week' },
  { value: 'lastMonth', labelKey: 'ui-export-last-month' },
  { value: 'all', labelKey: 'ui-export-all' },
] as const satisfies readonly { value: ExportPeriodChoice; labelKey: string }[]
/** Shown in the address field: a private address, as an own server usually has. */
const SERVER_EXAMPLE = '192.168.1.10'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()

// The form is filled once and then belongs to the user: answers never rewrite what is typed.
const form = ref<SettingsForm | null>(null)
const servers = ref<ServersChoice>(PUBLIC_SERVERS)
const typedServer = ref('')
const saved = ref('')
const refusal = ref('')
// The server last accepted: a switch or a look saved while an address is half typed keeps it.
let savedServer: string | null = null
// One save at a time, in the order of the changes: the last change is the one that stays.
let pending: Promise<void> = Promise.resolve()

const send = async (ownServer: string | null): Promise<void> => {
  if (!form.value) return
  const sent: SettingsForm = { ...form.value, ownServer }
  try {
    saved.value = await client.saveSettings(sent, readUtcOffsetSeconds())
    refusal.value = ''
    savedServer = ownServer
  } catch (error) {
    refusal.value = error instanceof Error ? error.message : String(error)
    saved.value = ''
  }
}

const queue = (ownServer: string | null): Promise<void> => {
  pending = pending.then(() => send(ownServer))
  return pending
}

const saveChange = (): Promise<void> => queue(savedServer)

const saveTypedServer = (): Promise<void> => queue(typedServer.value)

const exportPeriod = ref<ExportPeriodChoice>('lastMonth')
const isConfirmingErase = ref(false)
const isBusy = ref(false)
const isExporting = ref(false)
/** Empty while the export can be used; otherwise why not. Disabled until read. */
const exportUnavailable = ref<string | null>(null)
const isExportReady = ref(false)
/** The button that shows the export folder, once an export is done. */
const revealLabel = ref('')
const dataStatus = ref('')
const dataRefusal = ref('')

const runDataAction = async (action: () => Promise<string>): Promise<void> => {
  isBusy.value = true
  dataRefusal.value = ''
  dataStatus.value = ''
  try {
    dataStatus.value = await action()
  } catch (error) {
    dataRefusal.value = error instanceof Error ? error.message : String(error)
  } finally {
    isBusy.value = false
  }
}

const loadExportAvailability = async (): Promise<void> => {
  try {
    const availability = await client.readExportAvailability()
    isExportReady.value = availability.kind === 'ready'
    exportUnavailable.value = availability.kind === 'unavailable' ? availability.reason : null
  } catch (error) {
    console.error('Reading whether there is anything to export failed', error)
  }
}

// The folder first: a dialog closed without a choice exports nothing and says nothing.
const exportHistory = async (): Promise<void> => {
  revealLabel.value = ''
  await runDataAction(async () => {
    if (!(await client.chooseExportFolder())) return ''
    isExporting.value = true
    try {
      const done = await client.exportHistory(exportPeriod.value)
      revealLabel.value = done.revealLabel
      return done.message
    } finally {
      isExporting.value = false
    }
  })
}

const revealExport = async (): Promise<void> => {
  try {
    await client.revealExportFolder()
  } catch (error) {
    console.error('Showing the export folder failed', error)
  }
}

const eraseHistory = async (): Promise<void> => {
  isConfirmingErase.value = false
  revealLabel.value = ''
  await runDataAction(() => client.eraseHistory())
  await loadExportAvailability()
}

// The public servers need nothing more; an own server waits for its address.
const chooseServers = async (): Promise<void> => {
  refusal.value = ''
  if (servers.value === PUBLIC_SERVERS) await queue(null)
}

useLoadOnFocus(loadExportAvailability)

onMounted(async () => {
  try {
    const read = await client.readSettings()
    form.value = read
    savedServer = read.ownServer
    servers.value = read.ownServer === null ? PUBLIC_SERVERS : OWN_SERVER
    typedServer.value = read.ownServer ?? ''
  } catch (error) {
    console.error('Reading the settings failed', error)
  }
})
</script>

<template>
  <section v-if="form" class="settings" aria-labelledby="settings-title">
    <h2 id="settings-title">{{ say('ui-settings') }}</h2>

    <div class="group">
      <h3>{{ say('ui-settings-general') }}</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-launch">{{ say('ui-settings-launch') }}</label>
          <input
            id="setting-launch"
            v-model="form.launchAtLogin"
            type="checkbox"
            role="switch"
            aria-describedby="setting-launch-hint"
            @change="saveChange"
          />
          <span id="setting-launch-hint" class="hint">{{ say('ui-settings-launch-hint') }}</span>
        </div>
        <div class="setting">
          <span id="setting-appearance" class="label">{{ say('ui-settings-appearance') }}</span>
          <div class="radios" role="radiogroup" aria-labelledby="setting-appearance">
            <label v-for="appearance in APPEARANCES" :key="appearance.value">
              <input
                v-model="form.appearance"
                type="radio"
                name="appearance"
                :value="appearance.value"
                @change="saveChange"
              />
              {{ say(appearance.labelKey) }}
            </label>
          </div>
          <span class="hint">{{ say('ui-settings-appearance-hint') }}</span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>{{ say('ui-settings-measure') }}</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-servers">{{ say('ui-settings-servers') }}</label>
          <select id="setting-servers" v-model="servers" @change="chooseServers">
            <option :value="PUBLIC_SERVERS">{{ say('ui-servers-public') }}</option>
            <option :value="OWN_SERVER">{{ say('ui-servers-own') }}</option>
          </select>
          <span class="hint">{{ say('ui-settings-servers-hint') }}</span>
        </div>
        <div v-if="servers === OWN_SERVER" class="setting">
          <label for="setting-server">{{ say('ui-settings-server') }}</label>
          <input
            id="setting-server"
            v-model.trim="typedServer"
            type="text"
            inputmode="decimal"
            autocomplete="off"
            spellcheck="false"
            :placeholder="say('ui-settings-server-example', { address: SERVER_EXAMPLE })"
            aria-describedby="setting-server-hint"
            :aria-invalid="refusal !== ''"
            @change="saveTypedServer"
          />
          <span id="setting-server-hint" class="hint">{{ say('ui-settings-server-hint') }}</span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>{{ say('ui-settings-notifications') }}</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-notify">{{ say('ui-settings-notify') }}</label>
          <input
            id="setting-notify"
            v-model="form.notifiesOnDegradation"
            type="checkbox"
            role="switch"
            aria-describedby="setting-notify-hint"
            @change="saveChange"
          />
          <span id="setting-notify-hint" class="hint">{{ say('ui-settings-notify-hint') }}</span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>{{ say('ui-settings-throughput') }}</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-throughput">{{ say('ui-settings-throughput-allow') }}</label>
          <input
            id="setting-throughput"
            v-model="form.allowsThroughputTests"
            type="checkbox"
            role="switch"
            aria-describedby="setting-throughput-hint"
            @change="saveChange"
          />
          <span id="setting-throughput-hint" class="hint">{{ say('ui-settings-throughput-hint') }}</span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>{{ say('ui-settings-data') }}</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-export">{{ say('ui-settings-export') }}</label>
          <div class="actions">
            <select id="setting-export" v-model="exportPeriod" data-export-period>
              <option v-for="period in EXPORT_PERIODS" :key="period.value" :value="period.value">
                {{ say(period.labelKey) }}
              </option>
            </select>
            <button
              type="button"
              class="secondary"
              :disabled="isBusy || !isExportReady"
              :aria-busy="isExporting"
              data-export
              @click="exportHistory"
            >
              {{ isExporting ? say('ui-settings-exporting') : say('ui-settings-export-action') }}
            </button>
          </div>
          <span v-if="exportUnavailable === null" class="hint">{{ say('ui-settings-export-hint') }}</span>
          <span v-else class="hint" data-export-unavailable>{{ exportUnavailable }}</span>
        </div>
        <div class="setting">
          <span class="label">{{ say('ui-settings-erase') }}</span>
          <button type="button" class="secondary" :disabled="isBusy" data-erase @click="isConfirmingErase = true">
            {{ say('ui-settings-erase-action') }}
          </button>
          <div
            v-if="isConfirmingErase"
            class="warning"
            role="alertdialog"
            aria-labelledby="erase-warning"
            data-erase-warning
          >
            <p id="erase-warning">
              {{ say('ui-settings-erase-warning') }}
            </p>
            <div class="actions">
              <button type="button" class="secondary" data-erase-confirm @click="eraseHistory">
                {{ say('ui-settings-erase-confirm') }}
              </button>
              <button type="button" class="link" data-erase-cancel @click="isConfirmingErase = false">
                {{ say('ui-cancel') }}
              </button>
            </div>
          </div>
        </div>
      </div>
      <p v-if="dataRefusal" class="refused" role="alert" data-data-refused>{{ dataRefusal }}</p>
      <p class="saved" role="status" data-data-status>{{ dataStatus }}</p>
      <button v-if="revealLabel" type="button" class="link" data-reveal-export @click="revealExport">
        {{ revealLabel }}
      </button>
    </div>

    <p v-if="refusal" class="refused" role="alert" data-refused>{{ refusal }}</p>
    <p class="saved" role="status" data-saved>{{ saved }}</p>
  </section>
</template>

<style scoped>
.settings {
  max-width: 640px;
  margin-top: var(--s5);
}

h2 {
  margin: 0;
  font-size: var(--t-2xl);
  font-weight: 600;
  letter-spacing: -0.01em;
}

.group {
  margin-top: var(--s6);
}

h3 {
  margin: 0 0 var(--s2);
  color: var(--muted);
  font-size: var(--t-sm);
  font-weight: 600;
}

.rows {
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface);
}

.setting {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: var(--s1) var(--s4);
  align-items: center;
  padding: var(--s3) var(--s4);
}

.setting + .setting {
  border-top: 1px solid var(--line);
}

.setting label,
.label {
  font-weight: 500;
}

.hint {
  grid-column: 1;
  color: var(--muted);
  font-size: var(--t-xs);
}

select,
input[type='text'] {
  min-height: 28px;
  padding: 0 var(--s2);
  border: 1px solid var(--line);
  border-radius: var(--radius-nested);
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}

input[aria-invalid='true'] {
  border-color: var(--degraded);
}

input[type='checkbox'][role='switch'] {
  position: relative;
  width: 36px;
  height: 22px;
  margin: 0;
  border-radius: 11px;
  background: var(--line);
  cursor: pointer;
  appearance: none;
}

input[type='checkbox'][role='switch']::after {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--surface);
  content: '';
}

input[type='checkbox'][role='switch']:checked {
  background: var(--solid);
}

input[type='checkbox'][role='switch']:checked::after {
  left: 17px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s2);
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
  grid-column: 1 / -1;
  gap: var(--s2);
  padding: var(--s3);
  border: 1px solid var(--line);
  border-radius: var(--radius-nested);
}

.warning p {
  margin: 0;
}

.radios {
  display: inline-flex;
  gap: var(--s3);
}

.radios label {
  display: inline-flex;
  gap: var(--s1);
  align-items: center;
  font-weight: 400;
}

.refused {
  margin: var(--s4) 0 0;
  color: var(--ink);
  font-size: var(--t-sm);
}

.saved {
  min-height: 1em;
  margin: var(--s4) 0 0;
  color: var(--muted);
  font-size: var(--t-xs);
}

@media (prefers-reduced-motion: no-preference) {
  input[type='checkbox'][role='switch']::after {
    transition: left 0.15s ease;
  }
}

@media (max-width: 520px) {
  .setting {
    grid-template-columns: 1fr;
  }

  .hint {
    grid-column: auto;
  }
}
</style>

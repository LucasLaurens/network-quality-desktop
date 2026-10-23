<script setup lang="ts">
import { onMounted, ref } from 'vue'
import type { AppearanceChoice, SettingsForm } from '../bindings'
import { readErrorMessage, useNetworkQualityClient } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'
import { useInterfaceTexts } from '../composables/interface-texts'
import DataSettings from './DataSettings.vue'

const PUBLIC_SERVERS = 'public'
const OWN_SERVER = 'own'
type ServersChoice = typeof PUBLIC_SERVERS | typeof OWN_SERVER

const APPEARANCES = [
  { value: 'system', labelKey: 'ui-appearance-system' },
  { value: 'light', labelKey: 'ui-appearance-light' },
  { value: 'dark', labelKey: 'ui-appearance-dark' },
] as const satisfies readonly { value: AppearanceChoice; labelKey: string }[]

/** Shown in the address field: a private address, as an own server usually has. */
const SERVER_EXAMPLE = '192.168.1.10'

const client = useNetworkQualityClient()
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
    refusal.value = readErrorMessage(error)
    saved.value = ''
  }
}

const queue = (ownServer: string | null): Promise<void> => {
  pending = pending.then(() => send(ownServer))
  return pending
}

const saveChange = (): Promise<void> => queue(savedServer)

const saveTypedServer = (): Promise<void> => queue(typedServer.value)

// The public servers need nothing more; an own server waits for its address.
const chooseServers = async (): Promise<void> => {
  refusal.value = ''
  if (servers.value === PUBLIC_SERVERS) await queue(null)
}

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
  <section class="settings" aria-labelledby="settings-title">
    <h2 id="settings-title">{{ say('ui-settings') }}</h2>
    <template v-if="form">
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
    </template>

    <!-- Apart from the form: the history can be exported or erased even if the settings could not be read. -->
    <DataSettings />

    <p v-if="refusal" class="refused" role="alert" data-refused>{{ refusal }}</p>
    <p class="saved" role="status" data-saved>{{ saved }}</p>
  </section>
</template>

<style scoped src="./settings.css"></style>

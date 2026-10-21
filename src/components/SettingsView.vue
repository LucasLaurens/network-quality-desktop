<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { AppearanceChoice, SettingsForm } from '../bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'

const PUBLIC_SERVERS = 'public'
const OWN_SERVER = 'own'
type ServersChoice = typeof PUBLIC_SERVERS | typeof OWN_SERVER

const APPEARANCES = [
  { value: 'system', label: 'Système' },
  { value: 'light', label: 'Clair' },
  { value: 'dark', label: 'Sombre' },
] as const satisfies readonly { value: AppearanceChoice; label: string }[]

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

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
  <section v-if="form" class="settings" aria-labelledby="settings-title">
    <h2 id="settings-title">Réglages</h2>

    <div class="group">
      <h3>Général</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-launch">Lancer à l'ouverture de session</label>
          <input
            id="setting-launch"
            v-model="form.launchAtLogin"
            type="checkbox"
            role="switch"
            aria-describedby="setting-launch-hint"
            @change="saveChange"
          />
          <span id="setting-launch-hint" class="hint">L'état reste visible dans la barre des menus.</span>
        </div>
        <div class="setting">
          <span id="setting-appearance" class="label">Apparence</span>
          <div class="radios" role="radiogroup" aria-labelledby="setting-appearance">
            <label v-for="appearance in APPEARANCES" :key="appearance.value">
              <input
                v-model="form.appearance"
                type="radio"
                name="appearance"
                :value="appearance.value"
                @change="saveChange"
              />
              {{ appearance.label }}
            </label>
          </div>
          <span class="hint">Système suit le réglage clair ou sombre de l'ordinateur.</span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>Mesure</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-servers">Serveurs de mesure</label>
          <select id="setting-servers" v-model="servers" @change="chooseServers">
            <option :value="PUBLIC_SERVERS">Cloudflare et Google (recommandé)</option>
            <option :value="OWN_SERVER">Serveur personnalisé</option>
          </select>
          <span class="hint">Une petite requête par seconde pour mesurer latence, gigue et perte.</span>
        </div>
        <div v-if="servers === OWN_SERVER" class="setting">
          <label for="setting-server">Adresse du serveur</label>
          <input
            id="setting-server"
            v-model.trim="typedServer"
            type="text"
            inputmode="decimal"
            autocomplete="off"
            spellcheck="false"
            placeholder="ex. 192.168.1.10"
            aria-describedby="setting-server-hint"
            :aria-invalid="refusal !== ''"
            @change="saveTypedServer"
          />
          <span id="setting-server-hint" class="hint"
            >Une adresse IP qui répond au ping, ou sur le port 443. Seul ce serveur est alors mesuré.</span
          >
        </div>
      </div>
    </div>

    <div class="group">
      <h3>Notifications</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-notify">Prévenir quand la connexion se dégrade</label>
          <input
            id="setting-notify"
            v-model="form.notifiesOnDegradation"
            type="checkbox"
            role="switch"
            aria-describedby="setting-notify-hint"
            @change="saveChange"
          />
          <span id="setting-notify-hint" class="hint">Seulement si la dégradation dure plus de 10 secondes.</span>
        </div>
      </div>
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

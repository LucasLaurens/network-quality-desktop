<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { AppearanceChoice, ExportPeriodChoice, SettingsForm } from '../bindings'
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

const EXPORT_PERIODS = [
  { value: 'lastDay', label: '24 dernières heures' },
  { value: 'lastWeek', label: '7 derniers jours' },
  { value: 'lastMonth', label: '30 derniers jours' },
  { value: 'all', label: 'Tout' },
] as const satisfies readonly { value: ExportPeriodChoice; label: string }[]

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
const exportPeriod = ref<ExportPeriodChoice>('lastMonth')
const isErasing = ref(false)
const isBusy = ref(false)
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

const exportHistory = (): Promise<void> =>
  runDataAction(() => client.exportHistory(exportPeriod.value, readUtcOffsetSeconds()))

const eraseHistory = (): Promise<void> => {
  isErasing.value = false
  return runDataAction(() => client.eraseHistory())
}

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

    <div class="group">
      <h3>Tests de débit</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-throughput">Autoriser les tests de débit</label>
          <input
            id="setting-throughput"
            v-model="form.allowsThroughputTests"
            type="checkbox"
            role="switch"
            aria-describedby="setting-throughput-hint"
            @change="saveChange"
          />
          <span id="setting-throughput-hint" class="hint">
            Désactivé par défaut. Les tests passent par Cloudflare (gratuit, sans compte) et ne partent jamais
            seuls&nbsp;: uniquement sur un clic sur Test rapide ou Test complet.
          </span>
        </div>
      </div>
    </div>

    <div class="group">
      <h3>Données</h3>
      <div class="rows">
        <div class="setting">
          <label for="setting-export">Exporter l'historique</label>
          <div class="actions">
            <select id="setting-export" v-model="exportPeriod" data-export-period>
              <option v-for="period in EXPORT_PERIODS" :key="period.value" :value="period.value">
                {{ period.label }}
              </option>
            </select>
            <button type="button" class="secondary" :disabled="isBusy" data-export @click="exportHistory">
              Exporter
            </button>
          </div>
          <span class="hint">
            Trois fichiers CSV dans Téléchargements&nbsp;: mesures, événements, tests de débit. Sans adresse matérielle
            de la box.
          </span>
        </div>
        <div class="setting">
          <span class="label">Effacer l'historique</span>
          <button type="button" class="secondary" :disabled="isBusy" data-erase @click="isErasing = true">
            Effacer…
          </button>
          <div v-if="isErasing" class="warning" role="alertdialog" aria-labelledby="erase-warning" data-erase-warning>
            <p id="erase-warning">
              Toutes les mesures (minutes et heures), les événements (coupures, changements de réseau, veilles) et les
              tests de débit seront supprimés, sans retour possible. Les réglages sont conservés.
            </p>
            <div class="actions">
              <button type="button" class="secondary" data-erase-confirm @click="eraseHistory">
                Effacer l'historique
              </button>
              <button type="button" class="link" data-erase-cancel @click="isErasing = false">Annuler</button>
            </div>
          </div>
        </div>
      </div>
      <p v-if="dataRefusal" class="refused" role="alert" data-data-refused>{{ dataRefusal }}</p>
      <p class="saved" role="status" data-data-status>{{ dataStatus }}</p>
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

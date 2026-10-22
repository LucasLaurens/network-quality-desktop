<script setup lang="ts">
import { inject, onMounted, onUnmounted, provide, ref } from 'vue'
import type { DashboardRequested, ProfileChoice, WindowModeTexts } from './bindings'
import DashboardView from './components/DashboardView.vue'
import SettingsView from './components/SettingsView.vue'
import { HELP_REQUESTED_KEY, REQUESTED_TEST_KEY } from './composables/dashboard-requests'
import { useInterfaceTexts } from './composables/interface-texts'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'

// Remembered in the webview's own storage: a convenience, losing it only shows the note again.
const TRAY_HINT_DISMISSED_KEY = 'tray-hint-dismissed'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')
const say = useInterfaceTexts()

const version = ref('…')
const historyNotice = ref<string | null>(null)
const windowMode = ref<WindowModeTexts | null>(null)
const isTrayHintVisible = ref(false)
const PAGES = [
  { value: 'dashboard', labelKey: 'ui-dashboard' },
  { value: 'settings', labelKey: 'ui-settings' },
] as const
const page = ref<(typeof PAGES)[number]['value']>('dashboard')
const requestedTest = ref<ProfileChoice | null>(null)
provide(REQUESTED_TEST_KEY, requestedTest)
const helpRequested = ref(false)
provide(HELP_REQUESTED_KEY, helpRequested)
let stopListening = (): void => undefined

// The tray menu and the popover ask for a page, or for a test the dashboard runs. O(1).
const followRequest = (request: DashboardRequested): void => {
  if (request.kind === 'settings') {
    page.value = 'settings'
    return
  }
  page.value = 'dashboard'
  if (request.kind === 'help') {
    helpRequested.value = true
    return
  }
  requestedTest.value = request.profile
}

const isTrayHintDismissed = (): boolean => {
  try {
    return localStorage.getItem(TRAY_HINT_DISMISSED_KEY) !== null
  } catch {
    return false
  }
}

const dismissTrayHint = (): void => {
  isTrayHintVisible.value = false
  try {
    localStorage.setItem(TRAY_HINT_DISMISSED_KEY, new Date().toISOString())
  } catch {
    // Storage blocked: the note is hidden for this launch only.
  }
}

const openTrayExtensionPage = async (): Promise<void> => {
  await client.openTrayExtensionPage()
}

const quitApp = async (): Promise<void> => {
  await client.quitApp()
}

const loadWindowMode = async (): Promise<void> => {
  try {
    windowMode.value = await client.readWindowModeTexts()
    isTrayHintVisible.value = !isTrayHintDismissed()
  } catch (error) {
    console.error('Reading the window mode failed', error)
  }
}

const loadHistoryNotice = async (): Promise<void> => {
  try {
    historyNotice.value = await client.readHistoryNotice()
  } catch (error) {
    console.error('Reading the history notice failed', error)
  }
}

const loadVersion = async (): Promise<void> => {
  try {
    version.value = (await client.readAppInfo()).version
  } catch (error) {
    console.error('Reading the app version failed', error)
    version.value = say('ui-version-unknown')
  }
}

const listenRequests = async (): Promise<void> => {
  try {
    stopListening = await client.listenDashboardRequests(followRequest)
  } catch (error) {
    console.error('Listening to the tray menu failed', error)
  }
}

onMounted(async () => {
  await Promise.all([listenRequests(), loadWindowMode(), loadHistoryNotice(), loadVersion()])
})

onUnmounted(() => {
  stopListening()
})
</script>

<template>
  <main class="window">
    <aside v-if="windowMode && isTrayHintVisible" class="hint" data-tray-hint>
      <p>{{ windowMode.message }}</p>
      <div class="hint-actions">
        <button type="button" class="link" data-tray-hint-link @click="openTrayExtensionPage">
          {{ windowMode.linkLabel }}
        </button>
        <button type="button" class="link" data-tray-hint-dismiss @click="dismissTrayHint">
          {{ windowMode.dismissLabel }}
        </button>
      </div>
    </aside>
    <p v-if="historyNotice" class="hint" data-history-notice>{{ historyNotice }}</p>
    <header class="titlebar">
      <h1>{{ say('ui-app-title') }}</h1>
      <nav class="pages" :aria-label="say('ui-pages')">
        <button
          v-for="entry in PAGES"
          :key="entry.value"
          type="button"
          :aria-current="page === entry.value ? 'page' : undefined"
          :data-page="entry.value"
          @click="page = entry.value"
        >
          {{ say(entry.labelKey) }}
        </button>
      </nav>
    </header>
    <DashboardView v-if="page === 'dashboard'" @open-settings="page = 'settings'" />
    <SettingsView v-else />
    <footer class="foot">
      <span>{{ say('ui-version', { version }) }}</span>
      <button v-if="windowMode" type="button" class="link" data-quit @click="quitApp">
        {{ windowMode.quitLabel }}
      </button>
    </footer>
  </main>
</template>

<style scoped>
.window {
  max-width: var(--content-max);
  /* Centred: a maximised window keeps its column in the middle, not against the left edge. */
  margin-inline: auto;
  padding: var(--s6);
}

.titlebar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s4);
  align-items: center;
  justify-content: space-between;
}

.pages {
  display: inline-flex;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--sunken);
}

.pages button {
  min-height: 28px;
  padding: 0 var(--s3);
  border: 0;
  border-radius: var(--radius-nested);
  background: none;
  color: var(--muted);
  font: inherit;
  font-size: var(--t-sm);
  cursor: pointer;
}

.pages button[aria-current='page'] {
  background: var(--surface);
  box-shadow: 0 0 0 1px var(--line);
  color: var(--ink);
}

h1 {
  margin: 0;
  font-size: var(--t-2xl);
  font-weight: 600;
  letter-spacing: -0.01em;
}

.hint {
  margin: 0 0 var(--s5);
  padding: var(--s3) var(--s4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  color: var(--muted);
  font-size: var(--t-sm);
}

.hint p {
  margin: 0;
}

.hint-actions {
  display: flex;
  gap: var(--s4);
  margin-top: var(--s2);
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

.foot {
  display: flex;
  gap: var(--s4);
  align-items: baseline;
  margin-top: var(--s7);
  color: var(--muted);
  font-size: var(--t-xs);
}
</style>

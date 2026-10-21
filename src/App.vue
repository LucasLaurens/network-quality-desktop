<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { WindowModeTexts } from './bindings'
import DashboardView from './components/DashboardView.vue'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'

// Remembered in the webview's own storage: a convenience, losing it only shows the note again.
const TRAY_HINT_DISMISSED_KEY = 'tray-hint-dismissed'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const version = ref('…')
const historyNotice = ref<string | null>(null)
const windowMode = ref<WindowModeTexts | null>(null)
const isTrayHintVisible = ref(false)

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
    version.value = 'inconnue'
  }
}

onMounted(async () => {
  await Promise.all([loadWindowMode(), loadHistoryNotice(), loadVersion()])
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
    <h1>Qualité de connexion</h1>
    <DashboardView />
    <footer class="foot">
      <span>Version {{ version }}</span>
      <button v-if="windowMode" type="button" class="link" data-quit @click="quitApp">
        {{ windowMode.quitLabel }}
      </button>
    </footer>
  </main>
</template>

<style scoped>
.window {
  max-width: var(--content-max);
  padding: var(--s6);
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

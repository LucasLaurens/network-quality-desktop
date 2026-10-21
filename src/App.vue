<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import type { TrayHint } from './bindings'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'

// Remembered in the webview's own storage: a convenience, losing it only shows the note again.
const TRAY_HINT_DISMISSED_KEY = 'tray-hint-dismissed'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const version = ref('…')
const trayHint = ref<TrayHint | null>(null)

const isTrayHintDismissed = (): boolean => {
  try {
    return localStorage.getItem(TRAY_HINT_DISMISSED_KEY) !== null
  } catch {
    return false
  }
}

const dismissTrayHint = (): void => {
  trayHint.value = null
  try {
    localStorage.setItem(TRAY_HINT_DISMISSED_KEY, new Date().toISOString())
  } catch {
    // Storage blocked: the note is hidden for this launch only.
  }
}

const openTrayExtensionPage = async (): Promise<void> => {
  await client.openTrayExtensionPage()
}

onMounted(async () => {
  if (!isTrayHintDismissed()) trayHint.value = await client.readTrayHint()
  try {
    version.value = (await client.readAppInfo()).version
  } catch (error) {
    console.error('Reading the app version failed', error)
    version.value = 'inconnue'
  }
})
</script>

<template>
  <main class="window">
    <aside v-if="trayHint" class="hint" data-tray-hint>
      <p>{{ trayHint.message }}</p>
      <div class="hint-actions">
        <button type="button" class="link" data-tray-hint-link @click="openTrayExtensionPage">
          {{ trayHint.linkLabel }}
        </button>
        <button type="button" class="link" data-tray-hint-dismiss @click="dismissTrayHint">
          {{ trayHint.dismissLabel }}
        </button>
      </div>
    </aside>
    <h1>Qualité de connexion</h1>
    <p class="empty">La mesure de la connexion arrive dans une prochaine version.</p>
    <footer class="foot">Version {{ version }}</footer>
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

.empty {
  margin: var(--s4) 0 0;
  color: var(--muted);
  font-size: var(--t-lg);
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
  margin-top: var(--s7);
  color: var(--muted);
  font-size: var(--t-xs);
}
</style>

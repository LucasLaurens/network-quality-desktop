<script setup lang="ts">
import { inject, onMounted, ref } from 'vue'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'

const client = inject(NETWORK_QUALITY_CLIENT_KEY)
if (!client) throw new Error('NetworkQualityClient is not provided')

const version = ref('…')

onMounted(async () => {
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

.foot {
  margin-top: var(--s7);
  color: var(--muted);
  font-size: var(--t-xs);
}
</style>

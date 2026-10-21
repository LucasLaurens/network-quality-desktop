<script setup lang="ts">
import type { DisplayState } from '../bindings'

const { state, size = 16 } = defineProps<{ state: DisplayState; size?: number }>()
</script>

<!-- One shape per state, so the state never relies on colour alone (same shapes as the menu-bar dot). -->
<template>
  <svg :width="size" :height="size" viewBox="0 0 16 16" aria-hidden="true" :style="{ color: `var(--${state})` }">
    <circle v-if="state === 'solid'" cx="8" cy="8" r="7" fill="currentColor" />
    <template v-else-if="state === 'unstable'">
      <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M8 1.5 A6.5 6.5 0 0 1 8 14.5 Z" fill="currentColor" />
    </template>
    <path v-else-if="state === 'degraded'" d="M8 1 L15 14.5 L1 14.5 Z" fill="currentColor" />
    <template v-else-if="state === 'offline'">
      <circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M3.5 12.5 L12.5 3.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </template>
    <circle
      v-else
      cx="8"
      cy="8"
      r="6.5"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-dasharray="2.4 2.4"
    />
  </svg>
</template>

<script setup lang="ts">
import { useTemplateRef } from 'vue'
import type { HelpDisplay } from '../bindings'

const { help } = defineProps<{ help: HelpDisplay }>()

const details = useTemplateRef<HTMLDetailsElement>('details')

// Set on the element itself: once the user folds it, only the element knows it is closed.
const unfold = (): void => {
  if (details.value) details.value.open = true
}

defineExpose({ unfold })
</script>

<template>
  <details ref="details" class="help" data-help>
    <summary>{{ help.title }}</summary>
    <p>{{ help.why }}</p>
    <ol>
      <li v-for="(step, index) in help.steps" :key="index">
        {{ step.label }}
        <code>{{ step.command }}</code>
      </li>
    </ol>
    <p>{{ help.withoutPermission }}</p>
  </details>
</template>

<style scoped>
.help {
  margin-top: var(--s2);
  font-size: var(--t-sm);
}

summary {
  font-weight: 500;
  text-decoration: underline;
  cursor: pointer;
}

p,
ol {
  margin: var(--s2) 0 0;
}

ol {
  display: grid;
  gap: var(--s2);
  padding-left: var(--s4);
}

code {
  display: block;
  margin-top: var(--s1);
  padding: var(--s1) var(--s2);
  border-radius: var(--radius-nested);
  background: var(--sunken);
  font-size: var(--t-xs);
  /* Wrapped rather than scrolled: the popover is narrow, and a copy still yields one line. */
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  user-select: all;
}
</style>

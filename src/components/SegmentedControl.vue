<script setup lang="ts" generic="T extends string">
const { options, label } = defineProps<{ options: readonly { value: T; label: string }[]; label: string }>()
const selected = defineModel<T>({ required: true })
</script>

<template>
  <div class="segmented" role="group" :aria-label="label">
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      :aria-pressed="selected === option.value"
      :data-option="option.value"
      @click="selected = option.value"
    >
      {{ option.label }}
    </button>
  </div>
</template>

<style scoped>
.segmented {
  display: inline-flex;
  padding: 2px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--sunken);
}

button {
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

button[aria-pressed='true'] {
  background: var(--surface);
  box-shadow: 0 0 0 1px var(--line);
  color: var(--ink);
}
</style>

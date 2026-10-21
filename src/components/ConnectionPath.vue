<script setup lang="ts">
import type { PathDisplay, SegmentDisplay } from '../bindings'

const { path } = defineProps<{ path: PathDisplay }>()

// The colour repeats the state; the word "en cause" carries the meaning.
const paint = (segment: SegmentDisplay): Record<string, string> =>
  segment.isCut ? {} : { background: segment.state ? `var(--${segment.state})` : 'var(--line)' }
</script>

<template>
  <div class="path" role="img" :aria-label="path.description" data-path>
    <span class="node">{{ path.computer }}</span>
    <span class="seg" :class="{ fault: path.local.fault, cut: path.local.isCut }" :style="paint(path.local)" />
    <span class="node">{{ path.gateway }}</span>
    <span
      class="seg"
      :class="{ fault: path.internetSegment.fault, cut: path.internetSegment.isCut }"
      :style="paint(path.internetSegment)"
    />
    <span class="node">{{ path.internet }}</span>
    <span class="seg-label local" data-segment="local">
      {{ path.local.label }} <b v-if="path.local.fault" data-fault>{{ path.local.fault }}</b>
    </span>
    <span class="seg-label internet" data-segment="internet">
      {{ path.internetSegment.label }}
      <b v-if="path.internetSegment.fault" data-fault>{{ path.internetSegment.fault }}</b>
    </span>
  </div>
</template>

<style scoped>
.path {
  display: grid;
  grid-template-columns: auto 1fr auto 1fr auto;
  align-items: center;
  column-gap: var(--s2);
  row-gap: var(--s1);
  margin-top: var(--s4);
}

.node {
  font-size: var(--t-sm);
  white-space: nowrap;
}

.seg {
  height: 4px;
  border-radius: 2px;
}

.seg.fault {
  height: 6px;
}

.seg.cut {
  height: 0;
  border-top: 3px dashed var(--offline);
}

.seg-label {
  color: var(--muted);
  font-size: var(--t-xs);
  text-align: center;
}

.seg-label.local {
  grid-column: 1 / 3;
}

.seg-label.internet {
  grid-column: 4 / 6;
}

.seg-label b {
  color: var(--ink);
  white-space: nowrap;
  font-weight: 600;
}
</style>

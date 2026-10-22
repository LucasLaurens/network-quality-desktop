<script setup lang="ts">
import { computed, ref } from 'vue'
import type { EvolutionDisplay, SeriesDisplay } from '../bindings'
import { useInterfaceTexts } from '../composables/interface-texts'
import StateShape from './StateShape.vue'

const { evolution, series, ticks, label } = defineProps<{
  evolution: EvolutionDisplay
  series: SeriesDisplay
  /** Under the time axis: its start, its middle and its end. */
  ticks: readonly [string, string, string]
  label: string
}>()

const say = useInterfaceTexts()

const WIDTH = 820
const HEIGHT = 200
/** Room for the threshold labels left of the plot. */
const LEFT = 118
const BOTTOM = 24
const TOP = 8
/** Space above the Dégradée threshold, so it never touches the top. */
const HEADROOM = 1.15
const TICK_ANCHORS = ['start', 'middle', 'end'] as const

interface Point {
  index: number
  x: number
  y: number
}

const count = computed(() => series.values.length)

// O(n) for n ≤ 241 points.
const top = computed(() =>
  Math.max(...series.values.map((value) => value ?? 0), (series.degradedAbove ?? 0) * HEADROOM, Number.EPSILON),
)

const toX = (index: number): number => LEFT + ((index + 0.5) / Math.max(1, count.value)) * (WIDTH - LEFT)
const toY = (value: number): number => HEIGHT - BOTTOM - (value / top.value) * (HEIGHT - BOTTOM - TOP)
const toPosition = (fraction: number | null): number => LEFT + (fraction ?? 0) * (WIDTH - LEFT)

/** Consecutive measured points: the line breaks at every gap, never bridging it. O(n). */
const runs = computed<Point[][]>(() => {
  const found: Point[][] = []
  let current: Point[] = []
  series.values.forEach((value, index) => {
    if (value === null) {
      if (current.length > 0) found.push(current)
      current = []
      return
    }
    current.push({ index, x: toX(index), y: toY(value) })
  })
  if (current.length > 0) found.push(current)
  return found
})

const lines = computed(() =>
  runs.value
    .filter((run) => run.length > 1)
    .map((run) => run.map((point) => `${point.x.toFixed(1)},${point.y.toFixed(1)}`).join(' ')),
)
const lonePoints = computed(() => runs.value.flatMap((run) => (run.length === 1 ? run : [])))

const thresholds = computed(() => [
  { y: toY(series.unstableFrom ?? 0), label: series.unstableLabel },
  { y: toY(series.degradedAbove ?? 0), label: series.degradedLabel },
])

// O(s) for the four states.
const stateLabel = (index: number): string | null => {
  const state = evolution.states[index]
  return evolution.stateLabels.find((entry) => entry.state === state)?.label ?? null
}

const hovered = ref<number | null>(null)
const hoveredValue = computed(() => (hovered.value === null ? null : series.values[hovered.value]))

const followPointer = (event: PointerEvent): void => {
  const box = (event.currentTarget as SVGSVGElement).getBoundingClientRect()
  if (box.width === 0) return
  const x = ((event.clientX - box.left) / box.width) * WIDTH
  hovered.value =
    x < LEFT ? null : Math.min(count.value - 1, Math.max(0, Math.floor(((x - LEFT) / (WIDTH - LEFT)) * count.value)))
}

const stepWithKeys = (event: KeyboardEvent): void => {
  const step = { ArrowLeft: -1, ArrowRight: 1 }[event.key]
  if (step === undefined) return
  event.preventDefault()
  hovered.value = Math.min(count.value - 1, Math.max(0, (hovered.value ?? count.value - 1) + step))
}

const measuredIndexes = computed(() => series.values.flatMap((value, index) => (value === null ? [] : [index])))
</script>

<template>
  <p v-if="evolution.isEmpty" class="empty" data-chart-empty>
    {{ say('ui-chart-empty') }}
  </p>
  <div v-else class="chart">
    <div class="plot">
      <svg
        :viewBox="`0 0 ${String(WIDTH)} ${String(HEIGHT)}`"
        role="img"
        :aria-label="label"
        tabindex="0"
        data-chart
        @pointermove="followPointer"
        @pointerleave="hovered = null"
        @keydown="stepWithKeys"
        @blur="hovered = null"
      >
        <g v-for="threshold in thresholds" :key="threshold.label">
          <line :x1="LEFT" :x2="WIDTH" :y1="threshold.y" :y2="threshold.y" class="threshold" />
          <text class="axis" :x="LEFT - 8" :y="threshold.y + 4" text-anchor="end">{{ threshold.label }}</text>
        </g>
        <line :x1="LEFT" :x2="WIDTH" :y1="HEIGHT - BOTTOM" :y2="HEIGHT - BOTTOM" class="baseline" />
        <g v-for="change in evolution.networkChanges" :key="change.label + String(change.position)" data-network-change>
          <line
            :x1="toPosition(change.position)"
            :x2="toPosition(change.position)"
            :y1="TOP"
            :y2="HEIGHT - BOTTOM"
            class="change"
          />
          <text
            class="axis"
            :x="toPosition(change.position) + ((change.position ?? 0) > 0.6 ? -6 : 6)"
            :y="TOP + 12"
            :text-anchor="(change.position ?? 0) > 0.6 ? 'end' : 'start'"
          >
            {{ change.label }}
          </text>
        </g>
        <polyline v-for="(points, index) in lines" :key="index" :points="points" class="line" data-line />
        <circle v-for="point in lonePoints" :key="point.index" :cx="point.x" :cy="point.y" r="2.5" class="dot" />
        <g v-if="hovered !== null" data-crosshair>
          <line :x1="toX(hovered)" :x2="toX(hovered)" :y1="TOP" :y2="HEIGHT - BOTTOM" class="crosshair" />
          <circle
            v-if="hoveredValue !== null && hoveredValue !== undefined"
            :cx="toX(hovered)"
            :cy="toY(hoveredValue)"
            r="4"
            class="marker"
          />
        </g>
        <text
          v-for="(tick, index) in ticks"
          :key="tick"
          class="axis"
          :x="LEFT + (index * (WIDTH - LEFT)) / 2"
          :y="HEIGHT - 4"
          :text-anchor="TICK_ANCHORS[index]"
        >
          {{ tick }}
        </text>
      </svg>
      <div
        v-if="hovered !== null"
        class="tooltip"
        role="status"
        data-tooltip
        :style="{
          left: `${String((toX(hovered) / WIDTH) * 100)}%`,
          transform: `translateX(${toX(hovered) > WIDTH * 0.7 ? '-100%' : '0'})`,
        }"
      >
        <span class="time num">{{ evolution.times[hovered] }}</span>
        <strong class="num">{{ series.texts[hovered] ?? say('ui-chart-no-value') }}</strong>
        <span v-if="stateLabel(hovered)" class="state">{{ stateLabel(hovered) }}</span>
      </div>
    </div>
    <div class="band" :style="{ paddingLeft: `${String((LEFT / WIDTH) * 100)}%` }" aria-hidden="true" data-band>
      <span
        v-for="(state, index) in evolution.states"
        :key="index"
        :style="{ background: state ? `var(--${state})` : 'var(--zone)' }"
      />
    </div>
    <div class="legend">
      <span>{{ say('ui-chart-band') }}</span>
      <span v-for="entry in evolution.stateLabels" :key="entry.state">
        <StateShape :state="entry.state" :size="10" />{{ entry.label }}
      </span>
    </div>
    <p class="coverage" data-coverage>{{ evolution.coverage }}</p>
    <details class="values">
      <summary>{{ say('ui-chart-values') }}</summary>
      <table>
        <thead>
          <tr>
            <th>{{ say('ui-chart-start') }}</th>
            <th class="right">{{ say('ui-chart-value') }}</th>
            <th>{{ say('ui-chart-state') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="index in measuredIndexes" :key="index">
            <td class="num">{{ evolution.times[index] }}</td>
            <td class="right num">{{ series.texts[index] }}</td>
            <td>{{ stateLabel(index) }}</td>
          </tr>
        </tbody>
      </table>
    </details>
  </div>
</template>

<style scoped>
.plot {
  position: relative;
  max-width: 820px;
}

.plot > svg {
  display: block;
  width: 100%;
  height: auto;
  overflow: visible;
  touch-action: pan-y;
}

.axis {
  fill: var(--muted);
  font-size: 12px;
}

.threshold {
  stroke: var(--line);
  stroke-dasharray: 4 4;
}

.baseline {
  stroke: var(--line);
}

.change {
  stroke: var(--muted);
  stroke-dasharray: 2 3;
}

.line {
  fill: none;
  stroke: var(--ink);
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
}

.dot {
  fill: var(--ink);
}

.crosshair {
  stroke: var(--muted);
}

.marker {
  fill: var(--ink);
  stroke: var(--surface);
  stroke-width: 2;
}

.tooltip {
  position: absolute;
  top: 0;
  display: grid;
  gap: 2px;
  min-width: 120px;
  padding: var(--s2) var(--s3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface);
  box-shadow: var(--shadow);
  font-size: var(--t-sm);
  pointer-events: none;
}

.time,
.state {
  color: var(--muted);
  font-size: var(--t-xs);
}

.band {
  display: flex;
  max-width: 820px;
  height: 6px;
  margin-top: var(--s2);
}

.band span {
  flex: 1;
}

.legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--s4);
  margin-top: var(--s2);
  color: var(--muted);
  font-size: var(--t-xs);
}

.legend span {
  display: inline-flex;
  gap: 6px;
  align-items: center;
}

.coverage {
  margin: var(--s1) 0 0;
  color: var(--muted);
  font-size: var(--t-xs);
}

.values {
  margin-top: var(--s3);
  font-size: var(--t-sm);
}

.values summary {
  color: var(--muted);
  cursor: pointer;
}

.values table {
  margin-top: var(--s2);
  border-collapse: collapse;
}

.values th,
.values td {
  padding: 2px var(--s4) 2px 0;
  text-align: left;
}

.values th {
  color: var(--muted);
  font-weight: 500;
}

.right {
  text-align: right;
}

.num {
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 0;
  padding: var(--s5) 0;
  color: var(--muted);
}
</style>

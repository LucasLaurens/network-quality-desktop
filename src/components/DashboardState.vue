<script setup lang="ts">
import { inject, nextTick, ref, useTemplateRef, watch } from 'vue'
import type { ConnectionDisplay, OverviewDisplay } from '../bindings'
import { HELP_REQUESTED_KEY } from '../composables/dashboard-requests'
import ConnectionPath from './ConnectionPath.vue'
import HelpDetails from './HelpDetails.vue'
import StateShape from './StateShape.vue'

const { display, overview, updatedAt } = defineProps<{
  display: ConnectionDisplay
  overview: OverviewDisplay | null
  updatedAt: string
}>()

const helpRequested = inject(HELP_REQUESTED_KEY, ref(false))
const helpDetails = useTemplateRef<InstanceType<typeof HelpDetails>>('helpDetails')

// Consumed at once, so that the dashboard shown again later leaves the help as the user left it.
watch(
  helpRequested,
  async (asked) => {
    if (!asked) return
    helpRequested.value = false
    await nextTick()
    helpDetails.value?.unfold()
  },
  { immediate: true },
)
</script>

<template>
  <section class="answer" aria-labelledby="dashboard-state" :data-state="display.state">
    <div class="head">
      <StateShape :state="display.state" :size="24" />
      <h2 id="dashboard-state">{{ display.stateLabel }}</h2>
      <p class="where">
        <span v-if="display.network" data-network>{{ display.network.label }}, {{ display.network.connection }}</span>
        <span class="num">Mis à jour à {{ updatedAt }}</span>
      </p>
    </div>
    <p class="cause" aria-live="polite">{{ display.cause }}</p>
    <p v-if="display.action" class="action" data-action>{{ display.action }}</p>
    <HelpDetails v-if="display.help" ref="helpDetails" :help="display.help" />
    <ConnectionPath v-if="display.path" :path="display.path" class="path" />
    <dl v-if="overview" class="overview" aria-label="Vue d'ensemble">
      <div class="fact">
        <dt>Aujourd'hui</dt>
        <dd class="num">
          <span data-fact="today">{{ overview.today }}</span>
          <span v-if="overview.todayCoverage" class="coverage" data-coverage>{{ overview.todayCoverage }}</span>
        </dd>
      </div>
      <div class="fact">
        <dt>Coupures aujourd'hui</dt>
        <dd class="num" data-fact="outages">{{ overview.outages }}</dd>
      </div>
      <div class="fact">
        <dt>Pire moment</dt>
        <dd class="num" data-fact="worst">{{ overview.worstMoment }}</dd>
      </div>
      <div class="fact">
        <dt>Sur ce réseau depuis</dt>
        <dd class="num" data-fact="since">{{ overview.onNetworkSince }}</dd>
      </div>
    </dl>
  </section>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: var(--s3);
}

h2 {
  margin: 0;
  font-size: var(--t-2xl);
  font-weight: 600;
  letter-spacing: -0.01em;
}

.where {
  display: grid;
  margin: 0 0 0 auto;
  color: var(--muted);
  font-size: var(--t-sm);
  text-align: right;
}

.cause {
  max-width: 62ch;
  margin: var(--s2) 0 0;
  font-size: var(--t-lg);
}

.action {
  margin: var(--s1) 0 0;
  font-weight: 500;
}

.path {
  max-width: 640px;
  margin-top: var(--s5);
}

.overview {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--s4);
  margin: var(--s6) 0 0;
  padding: var(--s4) 0;
  border-top: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
}

.fact {
  display: grid;
  gap: var(--s1);
  align-content: start;
}

dt {
  color: var(--muted);
  font-size: var(--t-xs);
}

dd {
  margin: 0;
  font-weight: 600;
}

.num {
  font-variant-numeric: tabular-nums;
}

@media (max-width: 640px) {
  .overview {
    grid-template-columns: 1fr 1fr;
  }
}

.coverage {
  display: block;
  color: var(--muted);
  font-size: var(--t-xs);
  font-weight: 400;
}
</style>

<script setup lang="ts">
import type { NetworkRowDisplay, StateShares } from '../bindings'

const { networks } = defineProps<{ networks: NetworkRowDisplay[] }>()

const STATES = ['solid', 'unstable', 'degraded', 'offline'] as const satisfies readonly (keyof StateShares)[]
</script>

<template>
  <p v-if="networks.length === 0" class="empty" data-networks-empty>
    Chaque réseau utilisé apparaîtra ici, avec le temps passé dans chaque état.
  </p>
  <div v-else class="scroll">
    <table>
      <thead>
        <tr>
          <th>Réseau</th>
          <th>Temps par état</th>
          <th class="right">Latence p90</th>
          <th class="right">Perte</th>
          <th class="right">Durée mesurée</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="network in networks" :key="network.label" data-network-row>
          <td>
            {{ network.label }}<span v-if="network.isCurrent" class="tag" data-current>actuel</span>
            <div class="small">
              {{ network.connection
              }}<template v-if="network.isMostReliable">
                · <strong data-most-reliable>le plus fiable pour la visio</strong></template
              >
            </div>
          </td>
          <td>
            <div class="shares">
              <div class="stack" role="img" :aria-label="network.sharesLabel">
                <template v-for="state in STATES" :key="state">
                  <span
                    v-if="(network.shares[state] ?? 0) > 0"
                    :style="{ width: `${String(network.shares[state] ?? 0)}%`, background: `var(--${state})` }"
                  />
                </template>
              </div>
              <span class="num small solid">{{ network.solidLabel }}</span>
            </div>
          </td>
          <td class="right num">{{ network.latencyP90 }}</td>
          <td class="right num">{{ network.loss }}</td>
          <td class="right num" data-measured>{{ network.measured }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.scroll {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th,
td {
  padding: var(--s2) var(--s3) var(--s2) 0;
  border-bottom: 1px solid var(--line);
  text-align: left;
  vertical-align: top;
}

th {
  color: var(--muted);
  font-size: var(--t-xs);
  font-weight: 500;
}

.right {
  padding-left: var(--s3);
  text-align: right;
}

th:last-child,
td:last-child {
  padding-right: 0;
}

.tag {
  margin-left: var(--s2);
  color: var(--muted);
  font-size: var(--t-xs);
}

.small {
  color: var(--muted);
  font-size: var(--t-xs);
}

.shares {
  display: flex;
  gap: 10px;
  align-items: center;
}

.stack {
  display: flex;
  gap: 2px;
  width: 140px;
  height: 6px;
}

.stack span {
  height: 100%;
  border-radius: 2px;
}

.solid {
  color: var(--ink);
}

.num {
  font-variant-numeric: tabular-nums;
}

.empty {
  margin: 0;
  padding: var(--s5) 0;
  color: var(--muted);
}

@media (max-width: 640px) {
  .stack {
    width: 80px;
  }
}
</style>

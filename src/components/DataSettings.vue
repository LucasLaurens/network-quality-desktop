<script setup lang="ts">
import { ref } from 'vue'
import type { ExportPeriodChoice } from '../bindings'
import { useInterfaceTexts } from '../composables/interface-texts'
import { useLoadOnFocus } from '../composables/use-load-on-focus'
import { readErrorMessage, useNetworkQualityClient } from '../services/network-quality-client'

const EXPORT_PERIODS = [
  { value: 'lastDay', labelKey: 'ui-export-last-day' },
  { value: 'lastWeek', labelKey: 'ui-export-last-week' },
  { value: 'lastMonth', labelKey: 'ui-export-last-month' },
  { value: 'all', labelKey: 'ui-export-all' },
] as const satisfies readonly { value: ExportPeriodChoice; labelKey: string }[]

const client = useNetworkQualityClient()
const say = useInterfaceTexts()

const exportPeriod = ref<ExportPeriodChoice>('lastMonth')
const isConfirmingErase = ref(false)
const isBusy = ref(false)
const isExporting = ref(false)
/** Why the export cannot be used; empty while it can. Disabled until read. */
const exportUnavailable = ref('')
const isExportReady = ref(false)
/** The button that shows the export folder, once an export is done. */
const revealLabel = ref('')
const dataStatus = ref('')
const dataRefusal = ref('')

const runDataAction = async (action: () => Promise<string>): Promise<void> => {
  isBusy.value = true
  dataRefusal.value = ''
  dataStatus.value = ''
  try {
    dataStatus.value = await action()
  } catch (error) {
    dataRefusal.value = readErrorMessage(error)
  } finally {
    isBusy.value = false
  }
}

const loadExportAvailability = async (): Promise<void> => {
  try {
    const availability = await client.readExportAvailability()
    isExportReady.value = availability.kind === 'ready'
    exportUnavailable.value = availability.kind === 'unavailable' ? availability.reason : ''
  } catch (error) {
    console.error('Reading whether there is anything to export failed', error)
  }
}

// The folder first: a dialog closed without a choice exports nothing and says nothing.
const exportHistory = async (): Promise<void> => {
  revealLabel.value = ''
  await runDataAction(async () => {
    if (!(await client.chooseExportFolder())) return ''
    isExporting.value = true
    try {
      const done = await client.exportHistory(exportPeriod.value)
      revealLabel.value = done.revealLabel
      return done.message
    } finally {
      isExporting.value = false
    }
  })
}

const revealExport = async (): Promise<void> => {
  try {
    await client.revealExportFolder()
  } catch (error) {
    console.error('Showing the export folder failed', error)
  }
}

const eraseHistory = async (): Promise<void> => {
  isConfirmingErase.value = false
  revealLabel.value = ''
  await runDataAction(() => client.eraseHistory())
  await loadExportAvailability()
}

useLoadOnFocus(loadExportAvailability)
</script>

<template>
  <div class="group">
    <h3>{{ say('ui-settings-data') }}</h3>
    <div class="rows">
      <div class="setting">
        <label for="setting-export">{{ say('ui-settings-export') }}</label>
        <div class="actions">
          <select id="setting-export" v-model="exportPeriod" data-export-period>
            <option v-for="period in EXPORT_PERIODS" :key="period.value" :value="period.value">
              {{ say(period.labelKey) }}
            </option>
          </select>
          <button
            type="button"
            class="secondary"
            :disabled="isBusy || !isExportReady"
            :aria-busy="isExporting"
            data-export
            @click="exportHistory"
          >
            {{ isExporting ? say('ui-settings-exporting') : say('ui-settings-export-action') }}
          </button>
        </div>
        <span v-if="exportUnavailable === ''" class="hint">{{ say('ui-settings-export-hint') }}</span>
        <span v-else class="hint" data-export-unavailable>{{ exportUnavailable }}</span>
      </div>
      <div class="setting">
        <span class="label">{{ say('ui-settings-erase') }}</span>
        <button type="button" class="secondary" :disabled="isBusy" data-erase @click="isConfirmingErase = true">
          {{ say('ui-settings-erase-action') }}
        </button>
        <div
          v-if="isConfirmingErase"
          class="warning"
          role="alertdialog"
          aria-labelledby="erase-warning"
          data-erase-warning
        >
          <p id="erase-warning">
            {{ say('ui-settings-erase-warning') }}
          </p>
          <div class="actions">
            <button type="button" class="secondary" data-erase-confirm @click="eraseHistory">
              {{ say('ui-settings-erase-confirm') }}
            </button>
            <button type="button" class="link" data-erase-cancel @click="isConfirmingErase = false">
              {{ say('ui-cancel') }}
            </button>
          </div>
        </div>
      </div>
    </div>
    <p v-if="dataRefusal" class="refused" role="alert" data-data-refused>{{ dataRefusal }}</p>
    <p class="saved" role="status" data-data-status>{{ dataStatus }}</p>
    <button v-if="revealLabel" type="button" class="link" data-reveal-export @click="revealExport">
      {{ revealLabel }}
    </button>
  </div>
</template>

<style scoped src="./settings.css"></style>

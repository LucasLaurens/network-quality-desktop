import { onMounted, onUnmounted, ref, type InjectionKey, type Ref } from 'vue'
import type { ProfileChoice } from '../bindings'
import type { NetworkQualityClient } from '../services/network-quality-client'
import { readUtcOffsetSeconds } from '../services/time-zone'

export const THROUGHPUT_PROFILES = [
  { value: 'quick', label: 'Test rapide' },
  { value: 'full', label: 'Test complet' },
] as const satisfies readonly { value: ProfileChoice; label: string }[]

/** A test the tray menu asked the dashboard to run; consumed once started. */
export const REQUESTED_TEST_KEY: InjectionKey<Ref<ProfileChoice | null>> = Symbol('RequestedTest')

/**
 * Loads now, then again each time the window comes back: the popover is only hidden between two
 * openings, and a test may have run, or the setting changed, in another window meanwhile.
 */
export function useLoadOnFocus(load: () => Promise<void>): void {
  const reload = (): void => {
    void load()
  }
  onMounted(() => {
    window.addEventListener('focus', reload)
    reload()
  })
  onUnmounted(() => {
    window.removeEventListener('focus', reload)
  })
}

/**
 * One test at a time per window, as the dashboard and the popover both run it; the core refuses a
 * second one started elsewhere. The core decides whether to
 * ask first: it reads the connection at the click. `onDone` reloads what shows the result.
 */
export function useThroughputTest(client: NetworkQualityClient, onDone: () => Promise<void>) {
  const running = ref<ProfileChoice | null>(null)
  /** The test waiting for the user's go-ahead, where data counts, and what it may use. */
  const asked = ref<{ profile: ProfileChoice; warning: string } | null>(null)
  const refusal = ref('')

  // O(1), plus the test itself.
  const run = async (profile: ProfileChoice, isConfirmed: boolean): Promise<void> => {
    if (running.value) return
    asked.value = null
    refusal.value = ''
    running.value = profile
    try {
      const outcome = await client.runThroughputTest(profile, isConfirmed, readUtcOffsetSeconds())
      if (outcome.kind === 'needsConfirmation') {
        asked.value = { profile, warning: outcome.warning }
        return
      }
      await onDone()
    } catch (error) {
      refusal.value = error instanceof Error ? error.message : String(error)
    } finally {
      running.value = null
    }
  }

  const cancel = (): void => {
    asked.value = null
  }

  return { running, asked, refusal, run, cancel }
}

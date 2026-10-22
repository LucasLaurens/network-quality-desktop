import { onMounted, onUnmounted } from 'vue'

/**
 * Loads now, then again each time the window comes back: windows are only hidden between two
 * openings, and what they show may have changed meanwhile (a test run elsewhere, a setting, the
 * first measure kept).
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

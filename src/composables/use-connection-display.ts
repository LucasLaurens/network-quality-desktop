import { onMounted, onUnmounted, ref, type Ref } from 'vue'
import type { ConnectionDisplay } from '../bindings'
import type { NetworkQualityClient } from '../services/network-quality-client'

/** The latest display, kept current for as long as the component is mounted. */
export function useConnectionDisplay(client: NetworkQualityClient): Ref<ConnectionDisplay | null> {
  const display = ref<ConnectionDisplay | null>(null)
  // Nothing to stop until the subscription is made.
  let stopListening = (): void => undefined
  let unmounted = false

  onMounted(async () => {
    // Subscribe first, so no change emitted while the first read is in flight is lost.
    const stop = await client.listenConnectionDisplay((next) => {
      display.value = next
    })
    if (unmounted) {
      stop()
      return
    }
    stopListening = stop
    // A display pushed while this read was in flight is newer: never overwrite it.
    const initial = await client.readConnectionDisplay()
    display.value ??= initial
  })

  onUnmounted(() => {
    unmounted = true
    stopListening()
  })

  return display
}

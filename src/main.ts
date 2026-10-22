import { createApp } from 'vue'
import App from './App.vue'
import ConnectionPopover from './components/ConnectionPopover.vue'
import './styles/tokens.css'
import { INTERFACE_TEXTS_KEY } from './composables/interface-texts'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'
import { TauriNetworkQualityClient } from './services/tauri-network-quality-client'

// The popover window loads the same page with `?view=popover` (see tauri.conf.json).
const root = new URLSearchParams(window.location.search).get('view') === 'popover' ? ConnectionPopover : App
const client = new TauriNetworkQualityClient()

// The texts come first: a window never shows a word before it has the catalogue's.
client
  .readInterfaceTexts()
  .then((texts) => {
    createApp(root).provide(NETWORK_QUALITY_CLIENT_KEY, client).provide(INTERFACE_TEXTS_KEY, texts).mount('#app')
  })
  .catch((error: unknown) => {
    console.error('Reading the interface texts failed', error)
  })

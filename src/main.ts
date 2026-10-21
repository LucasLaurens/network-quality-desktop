import { createApp } from 'vue'
import App from './App.vue'
import ConnectionPopover from './components/ConnectionPopover.vue'
import './styles/tokens.css'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'
import { TauriNetworkQualityClient } from './services/tauri-network-quality-client'

// The popover window loads the same page with `?view=popover` (see tauri.conf.json).
const root = new URLSearchParams(window.location.search).get('view') === 'popover' ? ConnectionPopover : App

createApp(root).provide(NETWORK_QUALITY_CLIENT_KEY, new TauriNetworkQualityClient()).mount('#app')

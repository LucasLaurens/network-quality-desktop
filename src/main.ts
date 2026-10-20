import { createApp } from 'vue'
import App from './App.vue'
import './styles/tokens.css'
import { NETWORK_QUALITY_CLIENT_KEY } from './services/network-quality-client'
import { TauriNetworkQualityClient } from './services/tauri-network-quality-client'

createApp(App).provide(NETWORK_QUALITY_CLIENT_KEY, new TauriNetworkQualityClient()).mount('#app')

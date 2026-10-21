import { createApp } from 'vue'
import App from '../../src/App.vue'
import '../../src/styles/tokens.css'
import { NETWORK_QUALITY_CLIENT_KEY } from '../../src/services/network-quality-client'
import { InMemoryNetworkQualityClient, createConnectionDisplay } from '../fakes/in-memory-network-quality-client'

// `?scenario=window` shows the window mode note, as on a desktop without a tray.
const isWindowMode = new URLSearchParams(window.location.search).get('scenario') === 'window'
const windowModeTexts = isWindowMode
  ? {
      message:
        "Ce bureau n'affiche pas les icônes de la barre système : l'app reste ouverte dans une fenêtre. L'extension AppIndicator fait apparaître la pastille.",
      linkLabel: "Ouvrir la page de l'extension",
      dismissLabel: 'Masquer',
      quitLabel: 'Quitter',
    }
  : null
const client = new InMemoryNetworkQualityClient(undefined, createConnectionDisplay(), windowModeTexts)
createApp(App).provide(NETWORK_QUALITY_CLIENT_KEY, client).mount('#app')

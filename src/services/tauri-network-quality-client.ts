import { commands, events, type AppInfo, type ConnectionDisplay } from '../bindings'
import type { NetworkQualityClient } from './network-quality-client'

/** Production adapter: talks to the Rust core through the generated Tauri bindings. */
export class TauriNetworkQualityClient implements NetworkQualityClient {
  readAppInfo(): Promise<AppInfo> {
    return commands.readAppInfo()
  }

  readConnectionDisplay(): Promise<ConnectionDisplay> {
    return commands.readConnectionDisplay()
  }

  async listenConnectionDisplay(onChange: (display: ConnectionDisplay) => void): Promise<() => void> {
    return events.connectionDisplayChanged.listen((event) => {
      onChange(event.payload)
    })
  }

  openDashboard(): Promise<void> {
    return commands.openDashboard()
  }
}

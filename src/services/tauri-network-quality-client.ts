import { commands, type AppInfo } from '../bindings'
import type { NetworkQualityClient } from './network-quality-client'

/** Production adapter: talks to the Rust core through the generated Tauri bindings. */
export class TauriNetworkQualityClient implements NetworkQualityClient {
  readAppInfo(): Promise<AppInfo> {
    return commands.readAppInfo()
  }
}

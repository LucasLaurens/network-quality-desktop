import type { InjectionKey } from 'vue'
import type { AppInfo, ConnectionDisplay } from '../bindings'

/** Port between the Vue front end and the Rust core. Components never call Tauri directly. */
export interface NetworkQualityClient {
  readAppInfo(): Promise<AppInfo>
  readConnectionDisplay(): Promise<ConnectionDisplay>
  /** Calls `onChange` with every new display; resolves to the function that stops listening. */
  listenConnectionDisplay(onChange: (display: ConnectionDisplay) => void): Promise<() => void>
  openDashboard(): Promise<void>
}

export const NETWORK_QUALITY_CLIENT_KEY: InjectionKey<NetworkQualityClient> = Symbol('NetworkQualityClient')

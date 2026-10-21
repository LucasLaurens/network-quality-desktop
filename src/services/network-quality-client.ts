import type { InjectionKey } from 'vue'
import type { AppInfo } from '../bindings'

/** Port between the Vue front end and the Rust core. Components never call Tauri directly. */
export interface NetworkQualityClient {
  readAppInfo(): Promise<AppInfo>
}

export const NETWORK_QUALITY_CLIENT_KEY: InjectionKey<NetworkQualityClient> = Symbol('NetworkQualityClient')

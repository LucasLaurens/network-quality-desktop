import type { InjectionKey } from 'vue'
import type { AppInfo, ConnectionDisplay, WindowModeTexts } from '../bindings'

/** Port between the Vue front end and the Rust core. Components never call Tauri directly. */
export interface NetworkQualityClient {
  readAppInfo(): Promise<AppInfo>
  readConnectionDisplay(): Promise<ConnectionDisplay>
  /** Calls `onChange` with every new display; resolves to the function that stops listening. */
  listenConnectionDisplay(onChange: (display: ConnectionDisplay) => void): Promise<() => void>
  openDashboard(): Promise<void>
  closePopover(): Promise<void>
  /** The note shown when the desktop has no tray; `null` when the dot is visible. */
  readWindowModeTexts(): Promise<WindowModeTexts | null>
  openTrayExtensionPage(): Promise<void>
  quitApp(): Promise<void>
  /** Asks for the permission that reveals the Wi-Fi name, or opens the system settings. */
  revealNetworkName(): Promise<void>
}

export const NETWORK_QUALITY_CLIENT_KEY: InjectionKey<NetworkQualityClient> = Symbol('NetworkQualityClient')

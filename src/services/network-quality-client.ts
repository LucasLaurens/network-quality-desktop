import type { InjectionKey } from 'vue'
import type {
  AppInfo,
  ConnectionDisplay,
  DashboardSummaryDisplay,
  EvolutionDisplay,
  PeriodChoice,
  ProfileChoice,
  SettingsForm,
  ThroughputOutcome,
  ThroughputPanel,
  WindowModeTexts,
} from '../bindings'

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
  /** Why the history is not what it was, if something happened to it at launch. */
  readHistoryNotice(): Promise<string | null>
  /** The day so far and the networks of the last 30 days; the offset is the local time zone's. */
  readDashboardSummary(utcOffsetSeconds: number): Promise<DashboardSummaryDisplay>
  /** The chart of a period, every metric included. */
  readEvolution(period: PeriodChoice, utcOffsetSeconds: number): Promise<EvolutionDisplay>
  readSettings(): Promise<SettingsForm>
  /** Resolves to the confirmation to show; rejects with the sentence explaining the refusal. */
  saveSettings(form: SettingsForm, utcOffsetSeconds: number): Promise<string>
  readThroughputPanel(utcOffsetSeconds: number): Promise<ThroughputPanel>
  /**
   * Runs the test, or asks first where data counts (`isConfirmed` false); rejects with the
   * sentence explaining why it did not run.
   */
  runThroughputTest(profile: ProfileChoice, isConfirmed: boolean, utcOffsetSeconds: number): Promise<ThroughputOutcome>
}

export const NETWORK_QUALITY_CLIENT_KEY: InjectionKey<NetworkQualityClient> = Symbol('NetworkQualityClient')

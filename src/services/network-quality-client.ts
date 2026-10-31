import { inject, type InjectionKey } from 'vue'
import type {
  AppInfo,
  ConnectionDisplay,
  DashboardRequested,
  DashboardSummaryDisplay,
  EvolutionDisplay,
  LineDetails,
  ExportAvailability,
  ExportDone,
  ExportPeriodChoice,
  PeriodChoice,
  ProfileChoice,
  SettingsForm,
  TechnicalDetails,
  ThroughputOutcome,
  ThroughputPanel,
  WindowModeTexts,
} from '../bindings'

/** Every word of the windows, by catalogue key, read once from the core when a window opens. */
export type InterfaceTexts = Partial<Record<string, string>>

/** Port between the Vue front end and the Rust core. Components never call Tauri directly. */
export interface NetworkQualityClient {
  readAppInfo(): Promise<AppInfo>
  /** Every word of the windows, from the catalogue. */
  readInterfaceTexts(): Promise<InterfaceTexts>
  readConnectionDisplay(): Promise<ConnectionDisplay>
  /** Calls `onChange` with every new display; resolves to the function that stops listening. */
  listenConnectionDisplay(onChange: (display: ConnectionDisplay) => void): Promise<() => void>
  openDashboard(): Promise<void>
  /** Opens the settings page of the dashboard and closes the popover. */
  openSettings(): Promise<void>
  /** Calls `onRequest` when the tray menu or the popover asks the dashboard for something. */
  listenDashboardRequests(onRequest: (request: DashboardRequested) => void): Promise<() => void>
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
  /** One page of tests, from 0 for the newest; past the last page, the last one. */
  readThroughputPanel(utcOffsetSeconds: number, page: number): Promise<ThroughputPanel>
  /**
   * Runs the test, or asks first where data counts (`isConfirmed` false); rejects with the
   * sentence explaining why it did not run.
   */
  runThroughputTest(profile: ProfileChoice, isConfirmed: boolean, utcOffsetSeconds: number): Promise<ThroughputOutcome>
  /** Asks for the export folder; `false` when the dialog is closed without a choice. */
  chooseExportFolder(): Promise<boolean>
  /** Writes three CSV files into the chosen folder; rejects with why not. */
  exportHistory(period: ExportPeriodChoice): Promise<ExportDone>
  /** Whether there is anything to export, and why not. */
  readExportAvailability(): Promise<ExportAvailability>
  /** Shows the export folder in the system's file manager. */
  revealExportFolder(): Promise<void>
  /** Erases measures, events and throughput tests; the settings stay. */
  eraseHistory(): Promise<string>
  /** The connection, its Wi-Fi and the traffic seen: quick to read. */
  readTechnicalDetails(): Promise<TechnicalDetails>
  /** The box's line and the access estimate: the box may take a few seconds to answer. */
  readLineDetails(): Promise<LineDetails>
}

export const NETWORK_QUALITY_CLIENT_KEY: InjectionKey<NetworkQualityClient> = Symbol('NetworkQualityClient')

/** The client a component talks to the core through. */
export function useNetworkQualityClient(): NetworkQualityClient {
  const client = inject(NETWORK_QUALITY_CLIENT_KEY)
  if (!client) throw new Error('NetworkQualityClient is not provided')
  return client
}

/** What a refusal from the core says, whatever shape it came in. */
export function readErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

import {
  commands,
  events,
  type AppInfo,
  type ConnectionDisplay,
  type DashboardRequested,
  type DashboardSummaryDisplay,
  type EvolutionDisplay,
  type ExportAvailability,
  type ExportDone,
  type ExportPeriodChoice,
  type LineDetails,
  type PeriodChoice,
  type ProfileChoice,
  type SettingsForm,
  type TechnicalDetails,
  type ThroughputOutcome,
  type ThroughputPanel,
  type WindowModeTexts,
} from '../bindings'
import type { NetworkQualityClient } from './network-quality-client'
import type { InterfaceTexts } from '../composables/interface-texts'

/** Production adapter: talks to the Rust core through the generated Tauri bindings. */
export class TauriNetworkQualityClient implements NetworkQualityClient {
  readAppInfo(): Promise<AppInfo> {
    return commands.readAppInfo()
  }

  readInterfaceTexts(): Promise<InterfaceTexts> {
    return commands.readInterfaceTexts()
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

  openSettings(): Promise<void> {
    return commands.openSettings()
  }

  async listenDashboardRequests(onRequest: (request: DashboardRequested) => void): Promise<() => void> {
    return events.dashboardRequested.listen((event) => {
      onRequest(event.payload)
    })
  }

  closePopover(): Promise<void> {
    return commands.closePopover()
  }

  readWindowModeTexts(): Promise<WindowModeTexts | null> {
    return commands.readWindowModeTexts()
  }

  openTrayExtensionPage(): Promise<void> {
    return commands.openTrayExtensionPage()
  }

  quitApp(): Promise<void> {
    return commands.quitApp()
  }

  revealNetworkName(): Promise<void> {
    return commands.revealNetworkName()
  }

  readHistoryNotice(): Promise<string | null> {
    return commands.readHistoryNotice()
  }

  async readDashboardSummary(utcOffsetSeconds: number): Promise<DashboardSummaryDisplay> {
    return unwrap(await commands.readDashboardSummary(utcOffsetSeconds))
  }

  async readEvolution(period: PeriodChoice, utcOffsetSeconds: number): Promise<EvolutionDisplay> {
    return unwrap(await commands.readEvolution(period, utcOffsetSeconds))
  }

  readSettings(): Promise<SettingsForm> {
    return commands.readSettings()
  }

  async saveSettings(form: SettingsForm, utcOffsetSeconds: number): Promise<string> {
    return unwrap(await commands.saveSettings(form, utcOffsetSeconds))
  }

  async readThroughputPanel(utcOffsetSeconds: number): Promise<ThroughputPanel> {
    return unwrap(await commands.readThroughputPanel(utcOffsetSeconds))
  }

  async runThroughputTest(
    profile: ProfileChoice,
    isConfirmed: boolean,
    utcOffsetSeconds: number,
  ): Promise<ThroughputOutcome> {
    return unwrap(await commands.runThroughputTest(profile, isConfirmed, utcOffsetSeconds))
  }

  async chooseExportFolder(): Promise<boolean> {
    return unwrap(await commands.chooseExportFolder())
  }

  async exportHistory(period: ExportPeriodChoice): Promise<ExportDone> {
    return unwrap(await commands.exportHistory(period))
  }

  readExportAvailability(): Promise<ExportAvailability> {
    return commands.readExportAvailability()
  }

  revealExportFolder(): Promise<void> {
    return commands.revealExportFolder()
  }

  async eraseHistory(): Promise<string> {
    return unwrap(await commands.eraseHistory())
  }

  readTechnicalDetails(): Promise<TechnicalDetails> {
    return commands.readTechnicalDetails()
  }

  async readLineDetails(): Promise<LineDetails> {
    return unwrap(await commands.readLineDetails())
  }
}

type CommandResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: string }

/** A command that failed in Rust rejects here, as any other failed call. */
function unwrap<T>(result: CommandResult<T>): T {
  if (result.status === 'error') throw new Error(result.error)
  return result.data
}

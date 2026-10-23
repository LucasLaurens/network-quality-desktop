import { faker } from '@faker-js/faker'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import type { Component } from 'vue'
import type {
  AppInfo,
  ConnectionDisplay,
  DashboardRequested,
  DashboardSummaryDisplay,
  DisplayState,
  EvolutionDisplay,
  DetailSection,
  ExportAvailability,
  ExportDone,
  ExportPeriodChoice,
  HelpDisplay,
  LineDetails,
  MetricDisplay,
  NetworkRowDisplay,
  OverviewDisplay,
  PathDisplay,
  PeriodChoice,
  ProfileChoice,
  SegmentDisplay,
  SeriesDisplay,
  SettingsForm,
  TechnicalDetails,
  ThroughputOutcome,
  ThroughputPanel,
  ThroughputRow,
  WindowModeTexts,
} from '../../src/bindings'
import { INTERFACE_TEXTS_KEY } from '../../src/composables/interface-texts'
import {
  NETWORK_QUALITY_CLIENT_KEY,
  type InterfaceTexts,
  type NetworkQualityClient,
} from '../../src/services/network-quality-client'
import interfaceTexts from './interface-texts.json'

/** What a test sets on the fake core; anything left out is generated. */
export interface FakeClientOptions {
  appInfo?: AppInfo | undefined
  display?: ConnectionDisplay | undefined
  windowModeTexts?: WindowModeTexts | null | undefined
}

/** The settings of a first launch, as the core reads them. */
export function createSettingsForm(overrides: Partial<SettingsForm> = {}): SettingsForm {
  return {
    launchAtLogin: true,
    appearance: 'system',
    ownServer: null,
    notifiesOnDegradation: true,
    allowsThroughputTests: false,
    ...overrides,
  }
}

/** Mounts a window component with what `main.ts` gives it, and lets its first reads land. */
export async function mountWindow(component: Component, client: NetworkQualityClient): Promise<VueWrapper> {
  const wrapper = mount(component, { global: { provide: createWindowProvisions(client) } })
  await flushPromises()
  return wrapper
}

/** What `main.ts` gives a window: the client, and the catalogue's texts as the core renders them. */
export function createWindowProvisions(client: NetworkQualityClient): Record<symbol, unknown> {
  return { [NETWORK_QUALITY_CLIENT_KEY as symbol]: client, [INTERFACE_TEXTS_KEY as symbol]: interfaceTexts }
}

export function createMetricDisplay(overrides: Partial<MetricDisplay> = {}): MetricDisplay {
  return {
    name: faker.word.noun(),
    definition: faker.lorem.sentence(),
    method: faker.word.words(2),
    value: `${String(faker.number.int({ min: 1, max: 400 }))} ms`,
    gauge: {
      position: faker.number.float({ min: 0, max: 1 }),
      unstableFrom: 0.25,
      degradedAbove: 0.75,
      label: faker.lorem.sentence(),
    },
    hint: null,
    ...overrides,
  }
}

export function createSegmentDisplay(overrides: Partial<SegmentDisplay> = {}): SegmentDisplay {
  return {
    label: faker.lorem.words(2),
    fault: null,
    state: 'solid',
    isCut: false,
    ...overrides,
  }
}

export function createPathDisplay(overrides: Partial<PathDisplay> = {}): PathDisplay {
  return {
    computer: faker.lorem.word(),
    gateway: faker.lorem.word(),
    internet: faker.lorem.word(),
    local: createSegmentDisplay(),
    internetSegment: createSegmentDisplay(),
    description: faker.lorem.sentence(),
    ...overrides,
  }
}

export function createConnectionDisplay(overrides: Partial<ConnectionDisplay> = {}): ConnectionDisplay {
  return {
    state: 'solid',
    stateLabel: faker.word.adjective(),
    cause: faker.lorem.sentence(),
    action: null,
    detail: null,
    metrics: [createMetricDisplay(), createMetricDisplay(), createMetricDisplay()],
    openDashboardLabel: faker.lorem.words(3),
    network: { label: faker.company.name(), connection: faker.lorem.words(2), wifiDetails: null, revealLabel: null },
    path: createPathDisplay(),
    help: null,
    ...overrides,
  }
}

export function createHelpDisplay(): HelpDisplay {
  return {
    title: faker.lorem.words(4),
    why: faker.lorem.sentence(),
    steps: [
      { label: faker.lorem.words(3), command: `sudo ${faker.lorem.word()}` },
      { label: faker.lorem.words(3), command: `sudo ${faker.lorem.word()}` },
    ],
    withoutPermission: faker.lorem.sentence(),
  }
}

export function createWindowModeTexts(): WindowModeTexts {
  return {
    message: faker.lorem.sentence(),
    linkLabel: faker.lorem.words(3),
    dismissLabel: faker.word.verb(),
    quitLabel: faker.word.verb(),
  }
}

export function createDetailSection(): DetailSection {
  return {
    title: faker.word.words(2),
    rows: Array.from({ length: 2 }, () => ({ label: faker.word.noun(), value: faker.word.words(3) })),
  }
}

export function createThroughputRow(overrides: Partial<ThroughputRow> = {}): ThroughputRow {
  return {
    when: faker.date.recent().toISOString(),
    profile: faker.word.noun(),
    download: `${String(faker.number.int({ min: 5, max: 900 }))} Mbit/s`,
    upload: `${String(faker.number.int({ min: 1, max: 300 }))} Mbit/s`,
    loadedLatency: `${String(faker.number.int({ min: 10, max: 400 }))} ms`,
    data: `${String(faker.number.int({ min: 2, max: 150 }))} Mo`,
    ...overrides,
  }
}

export function createThroughputPanel(overrides: Partial<ThroughputPanel> = {}): ThroughputPanel {
  return { isAllowed: true, note: faker.lorem.sentence(), tests: [], ...overrides }
}

export function createOverviewDisplay(overrides: Partial<OverviewDisplay> = {}): OverviewDisplay {
  return {
    today: faker.lorem.words(4),
    todayCoverage: faker.lorem.words(5),
    outages: faker.lorem.words(2),
    worstMoment: faker.lorem.words(3),
    onNetworkSince: faker.lorem.words(2),
    ...overrides,
  }
}

export function createNetworkRowDisplay(overrides: Partial<NetworkRowDisplay> = {}): NetworkRowDisplay {
  const solid = faker.number.int({ min: 40, max: 90 })
  return {
    label: faker.company.name(),
    connection: faker.lorem.words(2),
    shares: { solid, unstable: 100 - solid, degraded: 0, offline: 0 },
    solidLabel: `${String(solid)} % Solide`,
    sharesLabel: faker.lorem.sentence(),
    latencyP90: `${String(faker.number.int({ min: 5, max: 300 }))} ms`,
    loss: `${String(faker.number.int({ min: 0, max: 5 }))} %`,
    measured: `${String(faker.number.int({ min: 1, max: 99 }))} h`,
    isCurrent: false,
    isMostReliable: false,
    ...overrides,
  }
}

export function createDashboardSummary(overrides: Partial<DashboardSummaryDisplay> = {}): DashboardSummaryDisplay {
  return { overview: createOverviewDisplay(), networks: [createNetworkRowDisplay()], ...overrides }
}

export function createSeriesDisplay(values: (number | null)[]): SeriesDisplay {
  return {
    values,
    texts: values.map((value) => (value === null ? null : `${String(Math.round(value))} ms`)),
    unstableFrom: 100,
    degradedAbove: 300,
    unstableLabel: faker.lorem.words(2),
    degradedLabel: faker.lorem.words(2),
  }
}

/** A chart whose latency, jitter and loss follow `values`, scaled apart so each series is recognisable. */
export function createEvolutionDisplay(
  values: (number | null)[] = Array.from({ length: 12 }, () => faker.number.int({ min: 10, max: 200 })),
): EvolutionDisplay {
  const states: (DisplayState | null)[] = values.map((value) => (value === null ? null : 'solid'))
  return {
    times: values.map(() => faker.date.recent().toISOString()),
    states,
    latency: createSeriesDisplay(values),
    jitter: createSeriesDisplay(values.map((value) => (value === null ? null : value / 10))),
    loss: createSeriesDisplay(values.map((value) => (value === null ? null : value / 100))),
    networkChanges: [],
    stateLabels: (['solid', 'unstable', 'degraded', 'offline'] as const).map((state) => ({
      state,
      label: faker.word.adjective(),
    })),
    coverage: faker.lorem.words(4),
    isEmpty: values.every((value) => value === null),
  }
}

/**
 * Serves prepared data, lets the test push new displays, and records the dashboard openings.
 * Reads can be held, to replay a read that answers after a newer display was pushed.
 */
export class InMemoryNetworkQualityClient implements NetworkQualityClient {
  dashboardOpenings = 0
  popoverClosings = 0
  extensionPageOpenings = 0
  quitRequests = 0
  nameRevealRequests = 0
  private isWindowModeReadFailing = false
  private historyNotice: string | null = null
  private listeners: ((display: ConnectionDisplay) => void)[] = []
  private isHoldingReads = false
  private heldReads: (() => void)[] = []
  private dashboardSummary = createDashboardSummary()
  private evolution = createEvolutionDisplay()
  readonly evolutionRequests: PeriodChoice[] = []
  readonly savedForms: SettingsForm[] = []
  readonly throughputRuns: ProfileChoice[] = []
  readonly exports: ExportPeriodChoice[] = []
  readonly exportConfirmation = faker.lorem.sentence()
  readonly eraseConfirmation = faker.lorem.sentence()
  erases = 0
  settingsOpenings = 0
  private requestListeners: ((request: DashboardRequested) => void)[] = []
  technicalDetailsReads = 0
  lineDetailsReads = 0
  technicalDetails: TechnicalDetails = {
    sections: [createDetailSection(), createDetailSection()],
    refreshSeconds: faker.number.int({ min: 2, max: 10 }),
  }
  readonly lineDetails: LineDetails = { section: createDetailSection(), note: faker.lorem.sentence() }
  /** Empty while the line is read. */
  private lineRefusal = ''
  exportRevealings = 0
  readonly revealLabel = faker.lorem.words(3)
  readonly nothingToExport = faker.lorem.sentence()
  private unavailableReason = ''
  private hasHistory = true
  private isFolderDialogClosed = false
  private isHoldingExports = false
  private releaseExport = (): void => undefined
  /** Empty while exports and erases succeed. */
  private dataRefusal = ''
  private throughputPanel = createThroughputPanel()
  private readonly nextThroughputRow = createThroughputRow()
  /** Empty while tests run. */
  private throughputRefusal = ''
  /** Empty where data does not count. */
  private throughputWarning = ''
  private isHoldingThroughput = false
  private releaseThroughputTest = (): void => undefined
  private settings: SettingsForm = createSettingsForm()
  /** Empty while saves are accepted. */
  private settingsRefusal = ''
  private readonly confirmation = faker.lorem.sentence()
  private isHoldingSaves = false
  private heldSaves: (() => void)[] = []

  private readonly appInfo: AppInfo
  private display: ConnectionDisplay
  private readonly windowModeTexts: WindowModeTexts | null

  constructor({
    appInfo = { name: faker.word.noun(), version: faker.system.semver() },
    display = createConnectionDisplay(),
    windowModeTexts = null,
  }: FakeClientOptions = {}) {
    this.appInfo = appInfo
    this.display = display
    this.windowModeTexts = windowModeTexts
  }

  get listenerCount(): number {
    return this.listeners.length
  }

  readAppInfo(): Promise<AppInfo> {
    return Promise.resolve(this.appInfo)
  }

  readInterfaceTexts(): Promise<InterfaceTexts> {
    return Promise.resolve(interfaceTexts)
  }

  readConnectionDisplay(): Promise<ConnectionDisplay> {
    const snapshot = this.display
    if (!this.isHoldingReads) return Promise.resolve(snapshot)
    return new Promise((resolve) => {
      this.heldReads.push(() => {
        resolve(snapshot)
      })
    })
  }

  holdReads(): void {
    this.isHoldingReads = true
  }

  releaseReads(): void {
    this.isHoldingReads = false
    this.heldReads.forEach((release) => {
      release()
    })
    this.heldReads = []
  }

  listenConnectionDisplay(onChange: (display: ConnectionDisplay) => void): Promise<() => void> {
    this.listeners.push(onChange)
    return Promise.resolve(() => {
      this.listeners = this.listeners.filter((listener) => listener !== onChange)
    })
  }

  openDashboard(): Promise<void> {
    this.dashboardOpenings += 1
    return Promise.resolve()
  }

  openSettings(): Promise<void> {
    this.settingsOpenings += 1
    return Promise.resolve()
  }

  listenDashboardRequests(onRequest: (request: DashboardRequested) => void): Promise<() => void> {
    this.requestListeners.push(onRequest)
    return Promise.resolve(() => {
      this.requestListeners = this.requestListeners.filter((listener) => listener !== onRequest)
    })
  }

  /** What the tray menu would send. */
  sendDashboardRequest(request: DashboardRequested): void {
    this.requestListeners.forEach((listener) => {
      listener(request)
    })
  }

  closePopover(): Promise<void> {
    this.popoverClosings += 1
    return Promise.resolve()
  }

  readWindowModeTexts(): Promise<WindowModeTexts | null> {
    if (this.isWindowModeReadFailing) return Promise.reject(new Error('the core did not answer'))
    return Promise.resolve(this.windowModeTexts)
  }

  failWindowModeRead(): void {
    this.isWindowModeReadFailing = true
  }

  quitApp(): Promise<void> {
    this.quitRequests += 1
    return Promise.resolve()
  }

  readHistoryNotice(): Promise<string | null> {
    return Promise.resolve(this.historyNotice)
  }

  setHistoryNotice(notice: string): void {
    this.historyNotice = notice
  }

  revealNetworkName(): Promise<void> {
    this.nameRevealRequests += 1
    return Promise.resolve()
  }

  openTrayExtensionPage(): Promise<void> {
    this.extensionPageOpenings += 1
    return Promise.resolve()
  }

  readDashboardSummary(): Promise<DashboardSummaryDisplay> {
    return Promise.resolve(this.dashboardSummary)
  }

  setDashboardSummary(summary: DashboardSummaryDisplay): void {
    this.dashboardSummary = summary
  }

  readEvolution(period: PeriodChoice): Promise<EvolutionDisplay> {
    this.evolutionRequests.push(period)
    return Promise.resolve(this.evolution)
  }

  setEvolution(evolution: EvolutionDisplay): void {
    this.evolution = evolution
  }

  readSettings(): Promise<SettingsForm> {
    return Promise.resolve({ ...this.settings })
  }

  /** Every save from now on is refused with `refusal`. */
  refuseSettings(refusal: string): void {
    this.settingsRefusal = refusal
  }

  saveSettings(form: SettingsForm): Promise<string> {
    this.savedForms.push({ ...form })
    const answer = (): Promise<string> => {
      if (this.settingsRefusal !== '') return Promise.reject(new Error(this.settingsRefusal))
      this.settings = { ...form }
      return Promise.resolve(this.confirmation)
    }
    if (!this.isHoldingSaves) return answer()
    return new Promise((resolve, reject) => {
      this.heldSaves.push(() => {
        answer().then(resolve, reject)
      })
    })
  }

  /** Saves wait until `releaseSaves`, as a server probe would. */
  holdSaves(): void {
    this.isHoldingSaves = true
  }

  releaseSaves(): void {
    this.isHoldingSaves = false
    this.heldSaves.forEach((release) => {
      release()
    })
    this.heldSaves = []
  }

  readConfirmation(): string {
    return this.confirmation
  }

  readThroughputPanel(): Promise<ThroughputPanel> {
    return Promise.resolve(this.throughputPanel)
  }

  setThroughputPanel(panel: ThroughputPanel): void {
    this.throughputPanel = panel
  }

  runThroughputTest(profile: ProfileChoice, isConfirmed: boolean): Promise<ThroughputOutcome> {
    if (this.throughputWarning !== '' && !isConfirmed) {
      return Promise.resolve({ kind: 'needsConfirmation', warning: this.throughputWarning })
    }
    this.throughputRuns.push(profile)
    const answer = (): Promise<ThroughputOutcome> => {
      if (this.throughputRefusal !== '') return Promise.reject(new Error(this.throughputRefusal))
      this.throughputPanel = { ...this.throughputPanel, tests: [this.nextThroughputRow, ...this.throughputPanel.tests] }
      return Promise.resolve({ kind: 'done', row: this.nextThroughputRow })
    }
    if (!this.isHoldingThroughput) return answer()
    return new Promise((resolve, reject) => {
      this.releaseThroughputTest = () => {
        answer().then(resolve, reject)
      }
    })
  }

  /** Asked before every test, as on a connection where data counts. */
  warnBeforeThroughputTests(warning: string): void {
    this.throughputWarning = warning
  }

  /** Every test is refused with `refusal`. */
  refuseThroughputTests(refusal: string): void {
    this.throughputRefusal = refusal
  }

  /** The running test waits until `finishThroughputTest`. */
  holdThroughputTests(): void {
    this.isHoldingThroughput = true
  }

  finishThroughputTest(): void {
    this.isHoldingThroughput = false
    this.releaseThroughputTest()
  }

  chooseExportFolder(): Promise<boolean> {
    if (this.dataRefusal !== '') return Promise.reject(new Error(this.dataRefusal))
    return Promise.resolve(!this.isFolderDialogClosed)
  }

  exportHistory(period: ExportPeriodChoice): Promise<ExportDone> {
    this.exports.push(period)
    const done: ExportDone = { message: this.exportConfirmation, revealLabel: this.revealLabel }
    if (!this.isHoldingExports) return Promise.resolve(done)
    return new Promise((resolve) => {
      this.releaseExport = () => {
        resolve(done)
      }
    })
  }

  readExportAvailability(): Promise<ExportAvailability> {
    if (this.unavailableReason !== '') return Promise.resolve({ kind: 'unavailable', reason: this.unavailableReason })
    if (!this.hasHistory) return Promise.resolve({ kind: 'unavailable', reason: this.nothingToExport })
    return Promise.resolve({ kind: 'ready' })
  }

  revealExportFolder(): Promise<void> {
    this.exportRevealings += 1
    return Promise.resolve()
  }

  eraseHistory(): Promise<string> {
    if (this.dataRefusal !== '') return Promise.reject(new Error(this.dataRefusal))
    this.erases += 1
    this.hasHistory = false
    return Promise.resolve(this.eraseConfirmation)
  }

  /** Nothing measured yet: there is nothing to export. */
  startWithEmptyHistory(): void {
    this.hasHistory = false
  }

  /** The first minute is kept: there is something to export from now on. */
  measureFirstMinute(): void {
    this.hasHistory = true
  }

  /** The history lives in memory for the session: it cannot be exported. */
  keepHistoryInMemory(reason: string): void {
    this.unavailableReason = reason
  }

  /** The user closes the folder dialog without choosing. */
  closeFolderDialog(): void {
    this.isFolderDialogClosed = true
  }

  /** The running export waits until `finishExport`. */
  holdExports(): void {
    this.isHoldingExports = true
  }

  finishExport(): void {
    this.releaseExport()
  }

  /** Every export and erase is refused with `refusal`. */
  refuseDataActions(refusal: string): void {
    this.dataRefusal = refusal
  }

  readTechnicalDetails(): Promise<TechnicalDetails> {
    this.technicalDetailsReads += 1
    return Promise.resolve(this.technicalDetails)
  }

  readLineDetails(): Promise<LineDetails> {
    this.lineDetailsReads += 1
    if (this.lineRefusal !== '') return Promise.reject(new Error(this.lineRefusal))
    return Promise.resolve(this.lineDetails)
  }

  /** Every reading of the line is refused with `refusal`. */
  refuseLineDetails(refusal: string): void {
    this.lineRefusal = refusal
  }

  readNextThroughputRow(): ThroughputRow {
    return this.nextThroughputRow
  }

  pushDisplay(display: ConnectionDisplay): void {
    this.display = display
    this.listeners.forEach((listener) => {
      listener(display)
    })
  }
}

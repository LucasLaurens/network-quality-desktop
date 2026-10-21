import { faker } from '@faker-js/faker'
import type {
  AppInfo,
  ConnectionDisplay,
  DashboardSummaryDisplay,
  DisplayState,
  EvolutionDisplay,
  DetailSection,
  ExportPeriodChoice,
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
import type { NetworkQualityClient } from '../../src/services/network-quality-client'

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
    ...overrides,
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
  technicalDetailsReads = 0
  lineDetailsReads = 0
  technicalDetails: TechnicalDetails = { sections: [createDetailSection(), createDetailSection()] }
  readonly lineDetails: LineDetails = { section: createDetailSection(), note: faker.lorem.sentence() }
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
  private settings: SettingsForm = {
    launchAtLogin: true,
    appearance: 'system',
    ownServer: null,
    notifiesOnDegradation: true,
    allowsThroughputTests: false,
  }
  /** Empty while saves are accepted. */
  private settingsRefusal = ''
  private readonly confirmation = faker.lorem.sentence()
  private isHoldingSaves = false
  private heldSaves: (() => void)[] = []

  constructor(
    private readonly appInfo: AppInfo = { name: faker.word.noun(), version: faker.system.semver() },
    private display: ConnectionDisplay = createConnectionDisplay(),
    private readonly windowModeTexts: WindowModeTexts | null = null,
  ) {}

  get listenerCount(): number {
    return this.listeners.length
  }

  readAppInfo(): Promise<AppInfo> {
    return Promise.resolve(this.appInfo)
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

  /** Every save is refused with `refusal` until `acceptSettings`. */
  refuseSettings(refusal: string): void {
    this.settingsRefusal = refusal
  }

  acceptSettings(): void {
    this.settingsRefusal = ''
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

  exportHistory(period: ExportPeriodChoice): Promise<string> {
    if (this.dataRefusal !== '') return Promise.reject(new Error(this.dataRefusal))
    this.exports.push(period)
    return Promise.resolve(this.exportConfirmation)
  }

  eraseHistory(): Promise<string> {
    if (this.dataRefusal !== '') return Promise.reject(new Error(this.dataRefusal))
    this.erases += 1
    return Promise.resolve(this.eraseConfirmation)
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
    return Promise.resolve(this.lineDetails)
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

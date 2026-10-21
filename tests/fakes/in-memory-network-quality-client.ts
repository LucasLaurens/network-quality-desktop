import { faker } from '@faker-js/faker'
import type { AppInfo, ConnectionDisplay, MetricDisplay, WindowModeTexts } from '../../src/bindings'
import type { NetworkQualityClient } from '../../src/services/network-quality-client'

export function createMetricDisplay(overrides: Partial<MetricDisplay> = {}): MetricDisplay {
  return {
    name: faker.word.noun(),
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

export function createConnectionDisplay(overrides: Partial<ConnectionDisplay> = {}): ConnectionDisplay {
  return {
    state: 'solid',
    stateLabel: faker.word.adjective(),
    cause: faker.lorem.sentence(),
    action: null,
    detail: null,
    metrics: [createMetricDisplay(), createMetricDisplay(), createMetricDisplay()],
    openDashboardLabel: faker.lorem.words(3),
    network: { label: faker.company.name(), connection: faker.lorem.words(2) },
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

/**
 * Serves prepared data, lets the test push new displays, and records the dashboard openings.
 * Reads can be held, to replay a read that answers after a newer display was pushed.
 */
export class InMemoryNetworkQualityClient implements NetworkQualityClient {
  dashboardOpenings = 0
  popoverClosings = 0
  extensionPageOpenings = 0
  quitRequests = 0
  private isWindowModeReadFailing = false
  private listeners: ((display: ConnectionDisplay) => void)[] = []
  private isHoldingReads = false
  private heldReads: (() => void)[] = []

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

  openTrayExtensionPage(): Promise<void> {
    this.extensionPageOpenings += 1
    return Promise.resolve()
  }

  pushDisplay(display: ConnectionDisplay): void {
    this.display = display
    this.listeners.forEach((listener) => {
      listener(display)
    })
  }
}

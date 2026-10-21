import { faker } from '@faker-js/faker'
import type { AppInfo, ConnectionDisplay, MetricDisplay } from '../../src/bindings'
import type { NetworkQualityClient } from '../../src/services/network-quality-client'

export function createMetricDisplay(overrides: Partial<MetricDisplay> = {}): MetricDisplay {
  return {
    name: faker.word.noun(),
    method: faker.word.words(2),
    value: `${String(faker.number.int({ min: 1, max: 400 }))} ms`,
    state: 'solid',
    gauge: { position: faker.number.float({ min: 0, max: 1 }), unstableFrom: 0.25, degradedAbove: 0.75 },
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
    ...overrides,
  }
}

/** Serves prepared data, lets the test push new displays, and records the dashboard openings. */
export class InMemoryNetworkQualityClient implements NetworkQualityClient {
  dashboardOpenings = 0
  private listeners: ((display: ConnectionDisplay) => void)[] = []

  constructor(
    private readonly appInfo: AppInfo = { name: 'app', version: faker.system.semver() },
    private display: ConnectionDisplay = createConnectionDisplay(),
  ) {}

  get listenerCount(): number {
    return this.listeners.length
  }

  readAppInfo(): Promise<AppInfo> {
    return Promise.resolve(this.appInfo)
  }

  readConnectionDisplay(): Promise<ConnectionDisplay> {
    return Promise.resolve(this.display)
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

  pushDisplay(display: ConnectionDisplay): void {
    this.display = display
    this.listeners.forEach((listener) => {
      listener(display)
    })
  }
}

import type { AppInfo } from '../../src/bindings'
import type { NetworkQualityClient } from '../../src/services/network-quality-client'

export class InMemoryNetworkQualityClient implements NetworkQualityClient {
  constructor(private readonly appInfo: AppInfo) {}

  readAppInfo(): Promise<AppInfo> {
    return Promise.resolve(this.appInfo)
  }
}

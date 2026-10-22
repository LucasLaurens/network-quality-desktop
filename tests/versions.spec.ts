import { describe, expect, it } from 'vitest'
import cargoManifest from '../Cargo.toml?raw'
import packageManifest from '../package.json'
import releaseManifest from '../.release-please-manifest.json'
import tauriConfig from '../src-tauri/tauri.conf.json'

/** The `version` of `[workspace.package]`, the one every crate inherits. */
function readWorkspaceVersion(manifest: string): string | undefined {
  const section = manifest.split('[workspace.package]')[1]?.split('\n[')[0] ?? ''
  return /^version = "([^"]+)"/m.exec(section)?.[1]
}

describe('versions', () => {
  it('gives the app one version, in the release manifest, package.json and Cargo', () => {
    // Act
    const workspaceVersion = readWorkspaceVersion(cargoManifest)

    // Assert
    expect(packageManifest.version).toBe(releaseManifest['.'])
    expect(workspaceVersion).toBe(releaseManifest['.'])
  })

  it('lets the Tauri configuration take the version from Cargo', () => {
    // Act & Assert
    expect(tauriConfig).not.toHaveProperty('version')
  })
})

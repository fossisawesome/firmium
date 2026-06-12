import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export interface UpdateInfo {
  version: string
  currentVersion: string
  body?: string
}

// Holds the pending Update handle between checkForUpdate() and installUpdate()
// so callers don't need to manage the @tauri-apps/plugin-updater object themselves.
let pendingUpdate: Update | null = null

// Checks the configured update endpoint for a newer release.
// Returns null if already up to date, or on any network/parse error
// (Windows-only feature — the bundled deb/rpm builds don't carry update metadata).
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    const update = await check()
    if (!update?.available) return null
    pendingUpdate = update
    return { version: update.version, currentVersion: update.currentVersion, body: update.body }
  } catch (err) {
    console.warn('Update check failed:', err)
    return null
  }
}

// Downloads and installs the previously-found update, then relaunches the app.
export async function installUpdate(): Promise<void> {
  if (!pendingUpdate) throw new Error('No pending update — call checkForUpdate() first')
  await pendingUpdate.downloadAndInstall()
  await relaunch()
}

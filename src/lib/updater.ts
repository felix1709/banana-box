import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'

export interface AppUpdateResult {
  currentVersion: string
  latestVersion: string
  updateAvailable: boolean
}

export async function checkAppUpdate(): Promise<AppUpdateResult> {
  const update = await check()

  if (!update) {
    return {
      currentVersion: '',
      latestVersion: '',
      updateAvailable: false,
    }
  }

  return {
    currentVersion: update.currentVersion,
    latestVersion: update.version,
    updateAvailable: update.available,
  }
}

export async function installAppUpdate(): Promise<void> {
  const update = await check()
  if (!update?.available) return

  await update.downloadAndInstall()
  await relaunch()
}

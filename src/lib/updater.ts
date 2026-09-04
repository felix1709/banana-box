import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'

export const DAILY_UPDATE_CHECK_HOUR = 10
export const DAILY_UPDATE_CHECK_MINUTE = 0

export function nextDailyUpdateCheckDelay(now = new Date()) {
  const next = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate(),
    DAILY_UPDATE_CHECK_HOUR,
    DAILY_UPDATE_CHECK_MINUTE,
    0,
    0,
  )
  if (next.getTime() <= now.getTime()) next.setDate(next.getDate() + 1)
  return Math.max(0, next.getTime() - now.getTime())
}

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

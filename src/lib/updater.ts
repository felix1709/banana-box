import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'

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

export interface UpdateProgress {
  percent: number
  phase: 'downloading' | 'installing' | 'completed'
}

export interface UpdateInstallController {
  cancel: () => void
  done: Promise<void>
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

export function installAppUpdateWithProgress(
  onProgress?: (progress: UpdateProgress) => void,
): UpdateInstallController {
  let cancelled = false
  let update: Update | null = null

  const done = (async () => {
    update = await check()
    if (!update?.available) {
      onProgress?.({ percent: 100, phase: 'completed' })
      return
    }

    let downloadedBytes = 0
    let contentLength = 0

    await update.download((event: DownloadEvent) => {
      if (cancelled) return

      if (event.event === 'Started') {
        contentLength = event.data.contentLength ?? 0
        onProgress?.({ percent: 0, phase: 'downloading' })
        return
      }

      if (event.event === 'Progress') {
        downloadedBytes += event.data.chunkLength
        const percent = contentLength > 0
          ? Math.min(99, Math.max(1, Math.round((downloadedBytes / contentLength) * 100)))
          : 0
        onProgress?.({ percent, phase: 'downloading' })
        return
      }

      onProgress?.({ percent: 99, phase: 'downloading' })
    })

    if (cancelled) {
      await update?.close().catch(() => {})
      return
    }

    onProgress?.({ percent: 99, phase: 'installing' })
    await update.install()
    onProgress?.({ percent: 100, phase: 'completed' })
    await relaunch()
  })()

  return {
    cancel: () => {
      cancelled = true
    },
    done: done.then(() => undefined),
  }
}

import { beforeEach, describe, expect, it, vi } from 'vitest'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { checkAppUpdate, installAppUpdate, nextDailyUpdateCheckDelay } from '@/lib/updater'

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}))

describe('updater', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns an available update from the Tauri updater plugin', async () => {
    vi.mocked(check).mockResolvedValue({
      currentVersion: '0.1.2',
      version: '0.1.3',
      available: true,
      downloadAndInstall: vi.fn(),
    } as unknown as Update)

    await expect(checkAppUpdate()).resolves.toEqual({
      currentVersion: '0.1.2',
      latestVersion: '0.1.3',
      updateAvailable: true,
    })
  })

  it('downloads, installs, and relaunches when an update exists', async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined)
    vi.mocked(check).mockResolvedValue({
      currentVersion: '0.1.2',
      version: '0.1.3',
      available: true,
      downloadAndInstall,
    } as unknown as Update)

    await installAppUpdate()

    expect(downloadAndInstall).toHaveBeenCalled()
    expect(relaunch).toHaveBeenCalled()
  })

  it('schedules the next 10:00 check from a morning timestamp', () => {
    const now = new Date(2026, 6, 20, 9, 30, 0)
    expect(nextDailyUpdateCheckDelay(now)).toBe(30 * 60_000)
  })

  it('rolls to the next day after 10:00', () => {
    const now = new Date(2026, 6, 20, 10, 0, 0)
    expect(nextDailyUpdateCheckDelay(now)).toBe(24 * 60 * 60_000)
  })
})

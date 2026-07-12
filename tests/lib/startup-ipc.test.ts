import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'
import { acknowledgeMigrationSummary, getStartupStatus } from '@/lib/startup-ipc'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('startup IPC', () => {
  it('sends empty command objects required by the authenticated startup commands', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ state: 'ready', migrationSummary: null })

    await getStartupStatus()
    await acknowledgeMigrationSummary()

    expect(invoke).toHaveBeenNthCalledWith(1, 'get_startup_status', {})
    expect(invoke).toHaveBeenNthCalledWith(2, 'acknowledge_migration_summary', {})
  })
})

import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSyncStatusStore } from '@/stores/syncStatus'

describe('sync status store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('pulls workspace tables and records the last sync time', async () => {
    const store = useSyncStatusStore()
    const client = clientMock([{ id: 'p1', revision: 1 }])

    await store.pullWorkspace(client as never, 'workspace-1')

    expect(store.state).toBe('synced')
    expect(store.lastSyncedAt).not.toBe('')
    expect(client.from).toHaveBeenCalledWith('projects')
    expect(client.from).toHaveBeenCalledWith('comments')
  })

  it('turns a failed pull into an actionable sync error', async () => {
    const store = useSyncStatusStore()
    const client = {
      from: vi.fn(() => ({
        select: vi.fn(() => ({
          eq: vi.fn(async () => ({ data: null, error: { message: 'network down' } })),
        })),
      })),
    }

    await store.pullWorkspace(client as never, 'workspace-1')

    expect(store.state).toBe('error')
    expect(store.error).toBe('network down')
  })
})

function clientMock(rows: unknown[]) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(async () => ({ data: rows, error: null })),
      })),
    })),
  }
}

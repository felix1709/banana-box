import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { usePresenceStore } from '@/stores/presence'

describe('presence store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('tracks and clears online collaborators from a realtime channel', async () => {
    const track = vi.fn(async () => 'ok')
    const unsubscribe = vi.fn(async () => 'ok')
    const channel = {
      on: vi.fn(() => channel),
      subscribe: vi.fn((callback: (status: string) => void) => {
        callback('SUBSCRIBED')
        return channel
      }),
      track,
      unsubscribe,
    }
    const client = { channel: vi.fn(() => channel) }
    const store = usePresenceStore()

    await store.joinWorkspace(client as never, {
      workspaceId: 'workspace-1',
      userId: 'user-1',
      email: 'a@example.com',
    })
    store.applyPresenceState({ user: [{ userId: 'user-2', email: 'b@example.com' }] })

    expect(track).toHaveBeenCalled()
    expect(store.onlineUsers.map((user) => user.userId)).toEqual(['user-2'])

    await store.leave()
    expect(unsubscribe).toHaveBeenCalled()
    expect(store.onlineUsers).toEqual([])
  })
})

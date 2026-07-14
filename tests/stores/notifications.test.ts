import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useNotificationsStore } from '@/stores/notifications'

describe('notifications store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('loads unread notifications for the current user', async () => {
    const store = useNotificationsStore()
    const client = {
      from: vi.fn(() => ({
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            is: vi.fn(() => ({
              order: vi.fn(async () => ({
                data: [{ id: 'n1', kind: 'mention', read_at: null, created_at: 'now' }],
                error: null,
              })),
            })),
          })),
        })),
      })),
    }

    await store.loadUnread(client as never, 'user-1')

    expect(store.unreadCount).toBe(1)
  })
})

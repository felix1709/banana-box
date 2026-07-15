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

  it('accepts a project invite notification and marks it as read', async () => {
    const store = useNotificationsStore()
    const client = {
      rpc: vi.fn(async () => ({
        data: { workspace_id: 'workspace-1', project_id: 'project-1', role: 'editor' },
        error: null,
      })),
      from: vi.fn(() => ({
        update: vi.fn(() => ({
          eq: vi.fn(async () => ({ data: [], error: null })),
        })),
      })),
    }

    const result = await store.acceptInviteNotification(client as never, 'notification-1', 'invite-1')

    expect(client.rpc).toHaveBeenCalledWith('accept_invite_by_id', { invite_id: 'invite-1' })
    expect(client.from).toHaveBeenCalledWith('notifications')
    expect(result).toEqual({ workspaceId: 'workspace-1', projectId: 'project-1', role: 'editor' })
  })

  it('reads invite acceptance data from Supabase table-returning RPC arrays', async () => {
    const store = useNotificationsStore()
    const client = {
      rpc: vi.fn(async () => ({
        data: [{ workspace_id: 'workspace-1', project_id: 'project-1', role: 'editor' }],
        error: null,
      })),
      from: vi.fn(() => ({
        update: vi.fn(() => ({
          eq: vi.fn(async () => ({ data: [], error: null })),
        })),
      })),
    }

    const result = await store.acceptInviteNotification(client as never, 'notification-1', 'invite-1')

    expect(result).toEqual({ workspaceId: 'workspace-1', projectId: 'project-1', role: 'editor' })
  })
})

import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMembersStore } from '@/stores/members'

describe('members store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('creates a workspace invite link with the selected role', async () => {
    const store = useMembersStore()
    const client = clientMock()

    const invite = await store.createInvite(client as never, {
      appOrigin: 'banana-box://invite',
      workspaceId: 'workspace-1',
      projectId: null,
      scopeType: 'workspace',
      role: 'commenter',
      email: 'friend@example.com',
      userId: 'user-1',
      token: 'token-123',
    })

    expect(invite.url).toContain('token-123')
    expect(invite.role).toBe('commenter')
    expect(client.from).toHaveBeenCalledWith('invites')
  })

  it('loads workspace members ordered by creation time', async () => {
    const store = useMembersStore()
    const client = {
      from: vi.fn(() => ({
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            order: vi.fn(async () => ({
              data: [{ workspace_id: 'workspace-1', user_id: 'user-2', role: 'viewer', created_at: 'now' }],
              error: null,
            })),
          })),
        })),
      })),
    }

    await store.loadMembers(client as never, 'workspace-1')

    expect(store.members[0].role).toBe('viewer')
  })

  it('accepts an invite token through the Supabase RPC', async () => {
    const store = useMembersStore()
    const client = {
      rpc: vi.fn(async () => ({
        data: { workspace_id: 'workspace-1', project_id: null, role: 'editor' },
        error: null,
      })),
    }

    const result = await store.acceptInvite(client as never, 'token-123')

    expect(client.rpc).toHaveBeenCalledWith('accept_invite', { invite_token: 'token-123' })
    expect(result.workspaceId).toBe('workspace-1')
  })
})

function clientMock() {
  return {
    from: vi.fn(() => ({
      insert: vi.fn(() => ({
        select: vi.fn(() => ({
          single: vi.fn(async () => ({ data: { id: 'invite-1' }, error: null })),
        })),
      })),
    })),
  }
}

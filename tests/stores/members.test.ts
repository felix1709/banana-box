import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMembersStore } from '@/stores/members'

describe('members store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('creates a project invite link with the selected role', async () => {
    const store = useMembersStore()
    const client = clientMock()

    const invite = await store.createInvite(client as never, {
      appOrigin: 'banana-box://invite',
      workspaceId: 'workspace-1',
      projectId: 'project-1',
      scopeType: 'project',
      role: 'commenter',
      email: 'friend@example.com',
      userId: 'user-1',
      token: 'token-123',
    })

    expect(invite.url).toContain('token-123')
    expect(invite.role).toBe('commenter')
    expect(invite.scopeType).toBe('project')
    expect(client.from).toHaveBeenCalledWith('invites')
  })

  it('resolves a six digit account or unique nickname before sending an invite notification', async () => {
    const store = useMembersStore()
    const client = profileLookupClient([
      { id: 'user-2', email: '000002@banana-box.local', display_name: '制片' },
    ])

    const accountRecipient = await store.resolveInviteRecipient(client as never, '000002')
    const nicknameRecipient = await store.resolveInviteRecipient(client as never, '制片')

    expect(accountRecipient).toEqual({
      id: 'user-2',
      email: '000002@banana-box.local',
      displayName: '制片',
    })
    expect(nicknameRecipient.id).toBe('user-2')
  })

  it('rejects duplicate nickname matches so the creator does not invite the wrong user', async () => {
    const store = useMembersStore()
    const client = profileLookupClient([
      { id: 'user-2', email: '000002@banana-box.local', display_name: '剪辑' },
      { id: 'user-3', email: '000003@banana-box.local', display_name: '剪辑' },
    ])

    await expect(store.resolveInviteRecipient(client as never, '剪辑')).rejects.toThrow('找到多个同名用户')
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

function profileLookupClient(rows: Array<{ id: string, email: string, display_name: string }>) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        or: vi.fn(() => ({
          limit: vi.fn(async () => ({ data: rows, error: null })),
        })),
      })),
    })),
  }
}

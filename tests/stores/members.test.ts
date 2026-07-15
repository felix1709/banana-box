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

  it('searches invite recipients by account or nickname and returns selectable matches', async () => {
    const store = useMembersStore()
    const client = profileLookupClient([
      { id: 'user-2', email: '000002@banana-box.local', display_name: '剪辑' },
      { id: 'user-3', email: '000003@banana-box.local', display_name: '剪辑助理' },
    ])

    const results = await store.searchInviteRecipients(client as never, '剪辑', 'user-1')

    expect(results).toEqual([
      { id: 'user-2', email: '000002@banana-box.local', displayName: '剪辑' },
      { id: 'user-3', email: '000003@banana-box.local', displayName: '剪辑助理' },
    ])
  })

  it('creates a project invite and sends a notification to the selected user', async () => {
    const store = useMembersStore()
    const client = inviteNotificationClient()

    const invite = await store.createProjectUserInvite(client as never, {
      workspaceId: 'workspace-1',
      projectId: 'project-1',
      role: 'editor',
      recipient: { id: 'user-2', email: '000002@banana-box.local', displayName: '剪辑' },
      userId: 'user-1',
    })

    expect(invite.id).toBe('invite-1')
    expect(client.tables.invites.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      project_id: 'project-1',
      scope_type: 'project',
      role: 'editor',
      email: '000002@banana-box.local',
      created_by: 'user-1',
      updated_by: 'user-1',
    }))
    expect(client.tables.notifications.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      recipient_user_id: 'user-2',
      actor_user_id: 'user-1',
      kind: 'invite',
      target_type: 'project_invite',
      target_id: 'invite-1',
      created_by: 'user-1',
      updated_by: 'user-1',
    }))
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

  it('reads invite token acceptance data from Supabase table-returning RPC arrays', async () => {
    const store = useMembersStore()
    const client = {
      rpc: vi.fn(async () => ({
        data: [{ workspace_id: 'workspace-1', project_id: 'project-1', role: 'editor' }],
        error: null,
      })),
    }

    const result = await store.acceptInvite(client as never, 'token-123')

    expect(result).toEqual({ workspaceId: 'workspace-1', projectId: 'project-1', role: 'editor' })
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

function inviteNotificationClient() {
  const tables = {
    invites: {
      insert: vi.fn(() => ({
        select: vi.fn(() => ({
          single: vi.fn(async () => ({ data: { id: 'invite-1' }, error: null })),
        })),
      })),
    },
    notifications: {
      insert: vi.fn(async () => ({ data: [], error: null })),
    },
  }
  return {
    tables,
    from: vi.fn((table: keyof typeof tables) => tables[table]),
  }
}

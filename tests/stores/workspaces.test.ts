import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useWorkspacesStore } from '@/stores/workspaces'

const personalName = `a \u7684\u4e2a\u4eba\u7a7a\u95f4`

function tableMock(result: unknown) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    single: vi.fn(async () => result),
    maybeSingle: vi.fn(async () => result),
    insert: vi.fn(() => query),
    upsert: vi.fn(() => query),
    update: vi.fn(() => query),
    order: vi.fn(async () => result),
  }
  return query
}

function rpcMock(result: unknown) {
  return vi.fn(async () => result)
}

function profileRow() {
  return {
    id: 'user-1',
    email: 'a@example.com',
    display_name: 'a',
    avatar_url: null,
    created_at: 'now',
    updated_at: 'now',
  }
}

function workspaceRow() {
  return {
    id: 'workspace-1',
    name: personalName,
    owner_id: 'user-1',
    created_at: 'now',
    updated_at: 'now',
  }
}

describe('workspaces store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('bootstraps profile and personal workspace through a single RPC', async () => {
    const profileQuery = tableMock({ data: profileRow(), error: null })
    const client = {
      from: vi.fn((table: string) => {
        if (table === 'profiles') return profileQuery
        return tableMock({ data: [], error: null })
      }),
      rpc: rpcMock({
        data: {
          profile: profileRow(),
          workspace: workspaceRow(),
        },
        error: null,
      }),
    }
    const store = useWorkspacesStore()

    await store.bootstrapForUser(client as never, { id: 'user-1', email: 'a@example.com' } as never)

    expect(client.rpc).toHaveBeenCalledWith('bootstrap_user_workspace', {
      workspace_name: personalName,
      user_email: 'a@example.com',
      user_display_name: 'a',
    })
    expect(profileQuery.upsert).not.toHaveBeenCalled()
    expect(store.profile?.id).toBe('user-1')
    expect(store.activeWorkspace?.id).toBe('workspace-1')
  })

  it('does not require direct profiles writes when no membership exists yet', async () => {
    const client = {
      from: vi.fn(() => tableMock({ data: [], error: null })),
      rpc: rpcMock({
        data: {
          profile: profileRow(),
          workspace: workspaceRow(),
        },
        error: null,
      }),
    }
    const store = useWorkspacesStore()

    await store.bootstrapForUser(client as never, { id: 'user-1', email: 'a@example.com' } as never)

    expect(store.activeWorkspace?.id).toBe('workspace-1')
    expect(client.rpc).toHaveBeenCalledWith('bootstrap_user_workspace', {
      workspace_name: personalName,
      user_email: 'a@example.com',
      user_display_name: 'a',
    })
    expect(client.from).not.toHaveBeenCalled()
  })

  it('always clears loading when workspace bootstrap fails', async () => {
    const client = {
      from: vi.fn(() => tableMock({ data: null, error: null })),
      rpc: rpcMock({ data: null, error: { message: 'permission denied' } }),
    }
    const store = useWorkspacesStore()

    await store.bootstrapForUser(client as never, { id: 'user-1', email: 'a@example.com' } as never)

    expect(store.loading).toBe(false)
    expect(store.error).toBe('permission denied')
  })

  it('updates the current profile display name', async () => {
    const profileQuery = tableMock({ data: { ...profileRow(), display_name: '小明' }, error: null })
    const client = {
      from: vi.fn((table: string) => {
        expect(table).toBe('profiles')
        return profileQuery
      }),
      rpc: vi.fn(),
    }
    const store = useWorkspacesStore()
    store.profile = mapProfileForTest(profileRow())

    await store.updateDisplayName(client as never, '小明')

    expect(profileQuery.update).toHaveBeenCalledWith({ display_name: '小明' })
    expect(store.profile?.displayName).toBe('小明')
  })
})

function mapProfileForTest(row: ReturnType<typeof profileRow>) {
  return {
    id: row.id,
    email: row.email,
    displayName: row.display_name,
    avatarUrl: row.avatar_url,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

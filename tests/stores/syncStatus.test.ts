import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSyncStatusStore } from '@/stores/syncStatus'
import { useProjectsStore } from '@/stores/projects'

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
  listProjects: vi.fn(),
  saveProjectWithStages: vi.fn(),
  setProjectPublic: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

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

  it('hydrates project cards from pulled cloud projects and stages', async () => {
    const store = useSyncStatusStore()
    const projects = useProjectsStore()
    const client = tableClientMock({
      projects: [{
        id: 'project-1',
        code: 'L36',
        version: 'v1',
        name: 'Shared project',
        file_display_ref: 'cloud/project',
        release_date: '2026-07-31',
        main_stage_key: 'storyboard',
        archived: false,
        owner_user_id: 'user-1',
        is_public: true,
        last_activity_summary: '邀请成员加入',
        last_activity_actor_name: '导演',
        created_at: '2026-07-11T08:00:00Z',
        updated_at: '2026-07-12T08:00:00Z',
      }],
      project_stages: [{
        id: 'stage-1',
        project_id: 'project-1',
        stage_key: 'storyboard',
        position: 0,
        start_date: '2026-07-01',
        end_date: '2026-07-08',
        progress: 65,
        updated_at: '2026-07-12T08:00:00Z',
      }],
    })

    await store.pullWorkspace(client as never, 'workspace-1')

    expect(projects.projects).toHaveLength(1)
    expect(projects.projects[0]).toMatchObject({
      id: 'project-1',
      code: 'L36',
      name: 'Shared project',
      ownerUserId: 'user-1',
      isPublic: true,
    })
    expect(projects.projects[0].stages[0]).toMatchObject({
      id: 'stage-1',
      stageKey: 'storyboard',
      progress: 65,
    })
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

function tableClientMock(rowsByTable: Record<string, unknown[]>) {
  return {
    from: vi.fn((table: string) => ({
      select: vi.fn(() => ({
        eq: vi.fn(async () => ({ data: rowsByTable[table] ?? [], error: null })),
      })),
    })),
  }
}

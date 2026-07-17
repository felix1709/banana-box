import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useCloudMigrationStore } from '@/stores/cloudMigration'
import { useLibraryStore } from '@/stores/library'
import { useProjectsStore } from '@/stores/projects'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'

describe('cloud migration store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('detects local prompts, projects, and the loaded daily plan after login', () => {
    const library = useLibraryStore()
    library.hydrate({
      version: 1,
      categories: [{ id: 'cat-1', name: '脚本', color: '#ffcc00', order: 0 }],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: 'cat-1',
        tags: ['tag'],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 2,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    useProjectsStore().hydrate([project()])
    const daily = useDailyTasksStore()
    daily.day = {
      id: 'day-1',
      localDate: '2026-07-13',
      settledAt: null,
      reportSnapshot: null,
      groups: [{
        id: 'group-1',
        code: 'L36',
        projectId: 'project-1',
        position: 0,
        tasks: [{
          id: 'task-1',
          title: '今日任务',
          progress: 20,
          note: '',
          investedMinutes: 30,
          reminderTime: '',
          reminderContent: '',
          position: 0,
          sourceTaskId: null,
          sourceSnapshotHash: null,
          createdAt: '2026-07-13T00:00:00Z',
          updatedAt: '2026-07-13T00:00:00Z',
        }],
      }],
    }

    const migration = useCloudMigrationStore()

    expect(migration.summary).toEqual({
      categories: 1,
      prompts: 1,
      projects: 1,
      dailyDays: 1,
      dailyTasks: 1,
      hasLocalData: true,
    })
  })

  it('uploads local data into the active workspace without deleting local records', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    auth.client = clientMock() as never
    const workspaces = useWorkspacesStore()
    workspaces.activeWorkspaceId = 'workspace-1'
    useLibraryStore().hydrate({
      version: 1,
      categories: [{ id: 'cat-1', name: '脚本', color: '#ffcc00', order: 0 }],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: 'cat-1',
        tags: ['tag'],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 2,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    useProjectsStore().hydrate([project()])
    const migration = useCloudMigrationStore()

    await migration.migrateNow()

    expect(migration.status).toBe('completed')
    expect(auth.client?.from).toHaveBeenCalledWith('prompt_categories')
    expect(auth.client?.from).toHaveBeenCalledWith('prompts')
    expect(auth.client?.from).toHaveBeenCalledWith('projects')
    expect(auth.client?.from).toHaveBeenCalledWith('project_stages')
    expect(useLibraryStore().library.prompts).toHaveLength(1)
  })

  it('does not upload shared prompt references as duplicate personal cloud prompts', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    auth.client = clientMock() as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [{
        id: 'local-ref-1',
        title: 'Shared Prompt',
        content: 'Use this prompt',
        categoryId: null,
        tags: ['shared'],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 2,
        sourceType: 'shared',
        sharedPromptId: 'shared-1',
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })

    await useCloudMigrationStore().migrateNow()

    expect(auth.client?.from).not.toHaveBeenCalledWith('prompts')
    expect(useCloudMigrationStore().status).toBe('completed')
  })

  it('loads cloud record counts for the active workspace and compares pending upload totals', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    auth.client = clientMock({
      prompt_categories: 1,
      prompts: 1,
      projects: 0,
      daily_task_days: 0,
      daily_tasks: 0,
    }) as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    useLibraryStore().hydrate({
      version: 1,
      categories: [{ id: 'cat-1', name: '脚本', color: '#ffcc00', order: 0 }],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: 'cat-1',
        tags: [],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 2,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    useProjectsStore().hydrate([project()])

    const migration = useCloudMigrationStore()
    await migration.loadCloudSummary()

    expect(auth.client?.from).toHaveBeenCalledWith('prompt_categories')
    expect(auth.client?.from).toHaveBeenCalledWith('prompts')
    expect(auth.client?.from).toHaveBeenCalledWith('projects')
    expect(auth.client?.from).toHaveBeenCalledWith('daily_task_days')
    expect(auth.client?.from).toHaveBeenCalledWith('daily_tasks')
    expect(migration.cloudSummary.projects).toBe(0)
    expect(migration.comparison.find((row) => row.key === 'projects')?.pendingUpload).toBe(1)
  })
})

function clientMock(counts: Record<string, number> = {}) {
  return {
    from: vi.fn((table: string) => ({
      upsert: vi.fn(() => ({ select: vi.fn(() => ({ single: vi.fn(async () => ({ data: {}, error: null })) })) })),
      insert: vi.fn(async () => ({ data: [], error: null })),
      delete: vi.fn(() => ({ eq: vi.fn(async () => ({ data: [], error: null })) })),
      select: vi.fn((_columns: string, _options: unknown) => ({
        eq: vi.fn((_column: string, _value: string) => ({
          is: vi.fn(async () => ({
            count: counts[table] ?? 0,
            error: null,
          })),
        })),
      })),
    })),
  }
}

function project(): Project {
  return {
    id: 'project-1',
    code: 'L36',
    version: 'v1',
    name: '短片',
    filePath: 'C:\\work\\L36',
    fileExists: true,
    releaseDate: '2026-07-31',
    mainStageKey: 'storyboard',
    archived: false,
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: `stage-${stage.key}`,
      stageKey: stage.key,
      position,
      startDate: '2026-07-01',
      endDate: '2026-07-08',
      progress: stage.key === 'storyboard' ? 65 : 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
}

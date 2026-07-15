import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ProjectEditor from '@/components/projects/ProjectEditor.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'

const createProject = vi.hoisted(() => vi.fn())

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject,
  deleteProject: vi.fn(),
  listProjects: vi.fn(),
  saveProjectWithStages: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('ProjectEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    createProject.mockReset()
  })

  it('creates all eight scheduled stages without manual main-stage or progress inputs', async () => {
    const store = useProjectsStore()
    store.openEditor(null)
    createProject.mockImplementation(async (input) => ({
      id: 'p1',
      ...input,
      fileExists: true,
      archived: false,
      createdAt: '2026-07-12T00:00:00Z',
      updatedAt: '2026-07-12T00:00:00Z',
      stages: input.stages.map((stage, position) => ({
        id: `${stage.stageKey}-id`,
        position,
        updatedAt: '2026-07-12T00:00:00Z',
        ...stage,
      })),
    }))
    const wrapper = mount(ProjectEditor)

    await wrapper.get('[data-field="code"]').setValue('L36')
    await wrapper.get('[data-field="version"]').setValue('v1')
    await wrapper.get('[data-field="name"]').setValue('三丽鸥短片')
    await wrapper.get('[data-field="file-path"]').setValue('C:\\work\\L36')
    await wrapper.get('[data-field="release-date"]').setValue('2026-07-31')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('.project-fields select').exists()).toBe(false)
    expect(wrapper.find('[data-stage-progress]').exists()).toBe(false)
    expect(createProject).toHaveBeenCalledWith(
      expect.objectContaining({
        code: 'L36',
        stages: expect.arrayContaining([
          expect.objectContaining({ stageKey: 'storyboard', progress: 0 }),
          expect.objectContaining({ stageKey: 'final_composite' }),
        ]),
      }),
    )
    expect(createProject.mock.calls[0][0]).not.toHaveProperty('mainStageKey')
    expect(createProject.mock.calls[0][0].stages).toHaveLength(8)
    expect(store.projects).toHaveLength(1)
    expect(store.projectEditorOpen).toBe(false)
  })

  it('creates a public project, uploads it, and notifies the invited account', async () => {
    const auth = useAuthStore()
    const client = cloudClientMock()
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = client as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.resolveInviteRecipient = vi.fn(async () => ({
      id: 'user-2',
      email: '000002@banana-box.local',
      displayName: '制片',
    }))
    members.createInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'editor',
      email: '000002@banana-box.local',
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=abc',
    }))
    const store = useProjectsStore()
    store.openEditor(null)
    createProject.mockImplementation(async (input) => ({
      id: 'p1',
      ...input,
      ownerUserId: input.ownerUserId,
      isPublic: false,
      lastActivitySummary: '',
      lastActivityActorName: '',
      fileExists: true,
      archived: false,
      createdAt: '2026-07-12T00:00:00Z',
      updatedAt: '2026-07-12T00:00:00Z',
      stages: input.stages.map((stage, position) => ({
        id: `${stage.stageKey}-id`,
        position,
        updatedAt: '2026-07-12T00:00:00Z',
        ...stage,
      })),
    }))
    store.setPublic = vi.fn(async (projectId: string, isPublic: boolean) => {
      const project = store.projects.find((item) => item.id === projectId)
      if (project) project.isPublic = isPublic
      return project as never
    })

    const wrapper = mount(ProjectEditor)

    await wrapper.get('[data-field="code"]').setValue('L36')
    await wrapper.get('[data-field="version"]').setValue('v1')
    await wrapper.get('[data-field="name"]').setValue('三丽鸥短片')
    await wrapper.get('[data-field="file-path"]').setValue('C:\\work\\L36')
    await wrapper.get('[data-field="release-date"]').setValue('2026-07-31')
    await wrapper.get('[data-field="share-public"]').setValue(true)
    await wrapper.get('[data-field="invite-identity"]').setValue('000002')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(members.resolveInviteRecipient).toHaveBeenCalledWith(client, '000002')
    expect(store.setPublic).toHaveBeenCalledWith('p1', true)
    expect(client.from).toHaveBeenCalledWith('projects')
    expect(client.from).toHaveBeenCalledWith('project_stages')
    expect(members.createInvite).toHaveBeenCalledWith(client, expect.objectContaining({
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'editor',
      email: '000002@banana-box.local',
      userId: 'user-1',
    }))
    expect(client.from).toHaveBeenCalledWith('notifications')
    expect(client.tables.notifications.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      recipient_user_id: 'user-2',
      actor_user_id: 'user-1',
      kind: 'invite',
      target_type: 'project_invite',
      target_id: 'invite-1',
    }))
    expect(store.projectEditorOpen).toBe(false)
  })
})

function cloudClientMock() {
  const tables = {
    projects: {
      upsert: vi.fn(async () => ({ data: [], error: null })),
    },
    project_stages: {
      upsert: vi.fn(async () => ({ data: [], error: null })),
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

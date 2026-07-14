import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectBoardPage from '@/components/projects/ProjectBoardPage.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
  listProjects: vi.fn().mockResolvedValue([]),
  saveProjectWithStages: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('ProjectBoardPage', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    setActivePinia(createPinia())
  })

  it('renders projects as responsive notes and opens the selected project schedule', async () => {
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mount(ProjectBoardPage)

    expect(wrapper.findAll('[data-stage-column]')).toHaveLength(0)
    expect(wrapper.findAll('[data-project-note]')).toHaveLength(1)
    expect(wrapper.get('[data-project-note="p1"]').text()).toContain('L36')
    expect(wrapper.get('[data-project-note="p1"]').text()).toContain('v1')
    expect(wrapper.get('[data-project-note="p1"]').text()).toContain('Sample project')
    expect(wrapper.get('[data-project-note="p1"]').text()).toContain('2026-07-31')
    expect(wrapper.find('[data-project-timeline="p1"]').exists()).toBe(false)

    await wrapper.get('[data-project-note="p1"]').trigger('click')

    expect(wrapper.find('[data-project-timeline="p1"]').exists()).toBe(true)

    await wrapper.get('[data-action="edit-selected-project"]').trigger('click')
    expect(store.projectEditorOpen).toBe(true)
    expect(store.editorProjectId).toBe('p1')
  })

  it('opens the existing project editor when double-clicking a project note', async () => {
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mount(ProjectBoardPage)

    await wrapper.get('[data-project-note="p1"]').trigger('dblclick')

    expect(store.projectEditorOpen).toBe(true)
    expect(store.editorProjectId).toBe('p1')
  })

  it('opens the existing invite form from a toolbar icon popover', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = {} as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.createInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'viewer',
      email: null,
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=abc',
    }))
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mount(ProjectBoardPage)

    expect(wrapper.find('.invite-dialog').exists()).toBe(false)

    await wrapper.get('[data-action="project-invite-menu"]').trigger('click')
    expect(document.body.querySelector('.project-invite-popover')).toBeTruthy()

    const inviteButton = document.body.querySelector('[data-action="create-invite"]') as HTMLButtonElement
    inviteButton.click()
    await flushPromises()

    expect(members.createInvite).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
      projectId: 'p1',
      scopeType: 'project',
    }))
    expect(document.body.textContent).toContain('banana-box://invite?token=abc')
  })

  it('orders unfinished project timelines by final release date and hides completed stage bars', async () => {
    const store = useProjectsStore()
    const early = project()
    Object.assign(early, { id: 'p-early', code: 'L12', releaseDate: '2026-07-20' })
    early.stages[0].progress = 100
    early.stages[1].progress = 35
    const late = project()
    Object.assign(late, { id: 'p-late', code: 'L36', releaseDate: '2026-08-02' })
    const complete = project()
    Object.assign(complete, { id: 'p-complete', code: 'L50', releaseDate: '2026-07-15' })
    complete.stages.forEach((stage) => { stage.progress = 100 })
    store.hydrate([late, complete, early])

    const wrapper = mount(ProjectBoardPage)

    await wrapper.get('[data-project-note="p-early"]').trigger('click')
    expect(wrapper.find('[data-project-timeline="p-early"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="storyboard"]').exists()).toBe(false)
    expect(wrapper.find('[data-stage-bar="first_cut"]').exists()).toBe(true)

    await wrapper.get('[data-project-note="p-late"]').trigger('click')
    expect(wrapper.find('[data-project-timeline="p-late"]').exists()).toBe(true)
  })
})

function project(): Project {
  return {
    id: 'p1',
    code: 'L36',
    version: 'v1',
    name: 'Sample project',
    filePath: 'C:\\work\\L36',
    fileExists: true,
    releaseDate: '2026-07-31',
    mainStageKey: 'storyboard',
    archived: false,
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: stage.key,
      stageKey: stage.key,
      position,
      startDate: `2026-07-${String(position + 1).padStart(2, '0')}`,
      endDate: `2026-07-${String(position + 8).padStart(2, '0')}`,
      progress: stage.key === 'storyboard' ? 65 : 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
}

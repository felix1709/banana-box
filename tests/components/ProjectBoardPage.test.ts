import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectBoardPage from '@/components/projects/ProjectBoardPage.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useProjectsStore } from '@/stores/projects'
import { useScheduleRequestsStore } from '@/stores/scheduleRequests'
import { useWorkspacesStore } from '@/stores/workspaces'
import { archiveProject, deleteProject, setProjectPublic } from '@/lib/productionIpc'

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
  listProjects: vi.fn().mockResolvedValue([]),
  saveProjectWithStages: vi.fn(),
  setProjectPublic: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('ProjectBoardPage', () => {
  let wrappers: VueWrapper[] = []

  beforeEach(() => {
    document.body.innerHTML = ''
    wrappers = []
    vi.clearAllMocks()
    setActivePinia(createPinia())
  })

  afterEach(() => {
    for (const wrapper of wrappers) {
      wrapper.unmount()
    }
  })

  function mountBoard() {
    const wrapper = mount(ProjectBoardPage)
    wrappers.push(wrapper)
    return wrapper
  }

  it('renders projects as responsive notes and opens the selected project schedule', async () => {
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mountBoard()

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

  it('moves project activity summaries into the project log popover', async () => {
    const store = useProjectsStore()
    store.hydrate([project({
      lastActivityActorName: '导演',
      lastActivitySummary: '修改了分镜进度 65% -> 80%',
    })])
    const wrapper = mountBoard()

    expect(wrapper.get('[data-project-note="p1"]').text()).not.toContain('修改了分镜进度')

    await wrapper.get('[data-action="project-log-menu"]').trigger('click')

    expect(document.body.querySelector('.project-log-popover')).toBeTruthy()
    expect(document.body.textContent).toContain('项目日志')
    expect(document.body.textContent).toContain('导演：修改了分镜进度 65% -> 80%')
    expect(document.body.textContent).toContain('L36')
  })

  it('opens the existing project editor when double-clicking a project note', async () => {
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mountBoard()

    await wrapper.get('[data-project-note="p1"]').trigger('dblclick')

    expect(store.projectEditorOpen).toBe(true)
    expect(store.editorProjectId).toBe('p1')
  })

  it('opens the existing invite form from a toolbar icon popover', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = cloudClientMock() as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.searchInviteRecipients = vi.fn(async () => [
      { id: 'user-2', email: '000002@banana-box.local', displayName: '剪辑' },
    ])
    members.createProjectUserInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'viewer',
      email: '000002@banana-box.local',
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=hidden',
    }))
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])
    const wrapper = mountBoard()

    expect(wrapper.find('.invite-dialog').exists()).toBe(false)

    await wrapper.get('[data-action="project-invite-menu"]').trigger('click')
    expect(document.body.querySelector('.project-invite-popover')).toBeTruthy()

    const searchInput = document.body.querySelector('[data-field="invite-search"]') as HTMLInputElement
    searchInput.value = '剪辑'
    searchInput.dispatchEvent(new Event('input'))
    const searchButton = document.body.querySelector('[data-action="search-invite-users"]') as HTMLButtonElement
    searchButton.click()
    await flushPromises()

    const addButton = document.body.querySelector('[data-action="add-invite-user"][data-user-id="user-2"]') as HTMLButtonElement
    addButton.click()
    await flushPromises()

    expect(members.createProjectUserInvite).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
      projectId: 'p1',
      recipient: expect.objectContaining({ id: 'user-2' }),
    }))
    expect(document.body.textContent).toContain('已发送')
  })

  it('shows a public collaboration badge and lets only the creator invite from public projects', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = cloudClientMock() as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    expect(wrapper.get('[data-project-public-badge="p1"]').exists()).toBe(true)
    expect(wrapper.get('[data-action="project-invite-menu"]').attributes('disabled')).toBeUndefined()
  })

  it('shows creator authority on project cards and opens project-scoped invites from the card', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    const client = cloudClientMock()
    auth.client = client as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.searchInviteRecipients = vi.fn(async () => [
      { id: 'user-2', email: '000002@banana-box.local', displayName: '剪辑' },
    ])
    members.createProjectUserInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'editor',
      email: '000002@banana-box.local',
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=hidden',
    }))
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    expect(wrapper.get('[data-project-owner-badge="p1"]').text()).toContain('发起人')

    await wrapper.get('[data-action="card-project-invite"][data-project-invite="p1"]').trigger('click')
    await flushPromises()
    expect(document.body.querySelector('.project-invite-popover')).toBeTruthy()

    const searchInput = document.body.querySelector('[data-field="invite-search"]') as HTMLInputElement
    searchInput.value = '剪辑'
    searchInput.dispatchEvent(new Event('input'))
    const searchButton = document.body.querySelector('[data-action="search-invite-users"]') as HTMLButtonElement
    searchButton.click()
    await flushPromises()

    const addButton = document.body.querySelector('[data-action="add-invite-user"][data-user-id="user-2"]') as HTMLButtonElement
    addButton.click()
    await flushPromises()

    expect(members.createProjectUserInvite).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
      projectId: 'p1',
      recipient: expect.objectContaining({ id: 'user-2' }),
    }))
    expect(client.from).toHaveBeenCalledWith('projects')
    expect(client.from).toHaveBeenCalledWith('project_stages')
    expect(document.body.textContent).toContain('已发送')
  })

  it('shows the card invite entry on private projects and publishes before inviting', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = cloudClientMock() as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: false })])
    vi.mocked(setProjectPublic).mockResolvedValue(project({ ownerUserId: 'user-1', isPublic: true }))

    const wrapper = mountBoard()

    expect(wrapper.get('[data-action="card-project-invite"][data-project-invite="p1"]').exists()).toBe(true)

    await wrapper.get('[data-action="card-project-invite"][data-project-invite="p1"]').trigger('click')
    await flushPromises()

    expect(setProjectPublic).toHaveBeenCalledWith('p1', true)
    expect(document.body.querySelector('.project-invite-popover')).toBeTruthy()
  })

  it('blocks invite controls for private projects and for non-creators', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-2' } as never
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    expect(wrapper.find('[data-action="project-invite-menu"]').exists()).toBe(false)
    expect(wrapper.find('[data-action="card-project-invite"]').exists()).toBe(false)
  })

  it('does not show project metadata editing controls to collaborators', () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-2' } as never
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    expect(wrapper.find('[data-action="edit-selected-project"]').exists()).toBe(false)
    expect(wrapper.find('[data-action="toggle-project-public"]').exists()).toBe(false)
  })

  it('lets collaborators submit a schedule change request instead of editing project metadata', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-2' } as never
    auth.client = cloudClientMock() as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const requests = useScheduleRequestsStore()
    requests.createRequest = vi.fn(async () => ({ id: 'request-1' }))
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    await wrapper.get('[data-project-note="p1"]').trigger('click')
    await wrapper.get('[data-action="open-schedule-request"]').trigger('click')
    await wrapper.get('[data-field="schedule-request-stage"]').setValue('storyboard')
    await wrapper.get('[data-field="schedule-request-start"]').setValue('2026-07-03')
    await wrapper.get('[data-field="schedule-request-end"]').setValue('2026-07-12')
    await wrapper.get('[data-field="schedule-request-reason"]').setValue('分镜素材比预期晚两天到齐')
    await wrapper.get('[data-action="submit-schedule-request"]').trigger('submit')
    await flushPromises()

    expect(requests.createRequest).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
      workspaceId: 'workspace-1',
      projectId: 'p1',
      projectOwnerUserId: 'user-1',
      requesterUserId: 'user-2',
      stageKey: 'storyboard',
      requestedStartDate: '2026-07-03',
      requestedEndDate: '2026-07-12',
    }))
    expect(wrapper.text()).toContain('申请已提交')
  })

  it('orders unfinished project timelines by final release date and shows elapsed stage bars', async () => {
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

    const wrapper = mountBoard()

    await wrapper.get('[data-project-note="p-early"]').trigger('click')
    expect(wrapper.find('[data-project-timeline="p-early"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="storyboard"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="first_cut"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="final_composite"]').exists()).toBe(true)

    await wrapper.get('[data-project-note="p-late"]').trigger('click')
    expect(wrapper.find('[data-project-timeline="p-late"]').exists()).toBe(true)
  })

  it('shows active projects by default and switches to archived project management', async () => {
    const store = useProjectsStore()
    store.hydrate([
      project({ id: 'active-project', code: 'A01', archived: false }),
      project({ id: 'archived-project', code: 'Z99', archived: true }),
    ])

    const wrapper = mountBoard()

    expect(wrapper.find('[data-project-note="active-project"]').exists()).toBe(true)
    expect(wrapper.find('[data-project-note="archived-project"]').exists()).toBe(false)

    await wrapper.get('[data-action="toggle-archived-projects"]').trigger('click')

    expect(store.filters.archived).toBe(true)
    expect(wrapper.find('[data-project-note="active-project"]').exists()).toBe(false)
    expect(wrapper.find('[data-project-note="archived-project"]').exists()).toBe(true)
  })

  it('lets the project creator archive and restore projects from the board', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    const store = useProjectsStore()
    store.hydrate([project({ id: 'p1', ownerUserId: 'user-1', archived: false })])
    vi.mocked(archiveProject).mockImplementation(async (projectId, archived) => ({
      ...store.projects.find((item) => item.id === projectId)!,
      archived,
    }))
    const wrapper = mountBoard()

    await wrapper.get('[data-action="archive-project"][data-project-id="p1"]').trigger('click')
    await flushPromises()

    expect(archiveProject).toHaveBeenCalledWith('p1', true)
    expect(store.projects.find((item) => item.id === 'p1')?.archived).toBe(true)

    await wrapper.get('[data-action="toggle-archived-projects"]').trigger('click')
    await wrapper.get('[data-action="restore-project"][data-project-id="p1"]').trigger('click')
    await flushPromises()

    expect(archiveProject).toHaveBeenLastCalledWith('p1', false)
    expect(store.projects.find((item) => item.id === 'p1')?.archived).toBe(false)
  })

  it('lets only the project creator permanently delete archived projects after confirmation', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    const store = useProjectsStore()
    store.hydrate([project({ id: 'p1', ownerUserId: 'user-1', archived: true })])
    vi.mocked(deleteProject).mockResolvedValue(undefined)
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true)
    const wrapper = mountBoard()

    await wrapper.get('[data-action="toggle-archived-projects"]').trigger('click')
    await wrapper.get('[data-action="delete-archived-project"][data-project-id="p1"]').trigger('click')
    await flushPromises()

    expect(confirmSpy).toHaveBeenCalled()
    expect(deleteProject).toHaveBeenCalledWith('p1')
    expect(store.projects.find((item) => item.id === 'p1')).toBeUndefined()
  })

  it('does not show archive or delete controls to collaborators', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-2' } as never
    const store = useProjectsStore()
    store.hydrate([
      project({ id: 'active-project', ownerUserId: 'user-1', archived: false, isPublic: true }),
      project({ id: 'archived-project', ownerUserId: 'user-1', archived: true, isPublic: true }),
    ])
    const wrapper = mountBoard()

    expect(wrapper.find('[data-action="archive-project"]').exists()).toBe(false)

    await wrapper.get('[data-action="toggle-archived-projects"]').trigger('click')

    expect(wrapper.find('[data-action="restore-project"]').exists()).toBe(false)
    expect(wrapper.find('[data-action="delete-archived-project"]').exists()).toBe(false)
  })
})

function project(overrides: Partial<Project> = {}): Project {
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
    ownerUserId: 'user-1',
    isPublic: false,
    lastActivitySummary: '',
    lastActivityActorName: '',
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
    ...overrides,
  }
}

function cloudClientMock() {
  return {
    from: vi.fn((table: string) => ({
      upsert: vi.fn(async () => ({ data: [], error: null })),
      insert: vi.fn(() => ({
        select: vi.fn(() => ({
          single: vi.fn(async () => ({ data: { id: 'request-1' }, error: null })),
        })),
      })),
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn(() => ({
            eq: vi.fn(() => ({
              order: vi.fn(async () => ({ data: [], error: null })),
            })),
            order: vi.fn(async () => ({ data: [], error: null })),
          })),
          order: vi.fn(async () => ({ data: [], error: null })),
        })),
      })),
    })),
  }
}

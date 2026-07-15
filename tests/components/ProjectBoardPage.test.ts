import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectBoardPage from '@/components/projects/ProjectBoardPage.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'
import { setProjectPublic } from '@/lib/productionIpc'

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
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])
    const wrapper = mountBoard()

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
    members.createInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: 'p1',
      scopeType: 'project',
      role: 'editor',
      email: null,
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=card',
    }))
    const store = useProjectsStore()
    store.hydrate([project({ ownerUserId: 'user-1', isPublic: true })])

    const wrapper = mountBoard()

    expect(wrapper.get('[data-project-owner-badge="p1"]').text()).toContain('发起人')

    await wrapper.get('[data-action="card-project-invite"][data-project-invite="p1"]').trigger('click')
    await flushPromises()
    expect(document.body.querySelector('.project-invite-popover')).toBeTruthy()

    const inviteButton = document.body.querySelector('[data-action="create-invite"]') as HTMLButtonElement
    inviteButton.click()
    await flushPromises()

    expect(members.createInvite).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({
      projectId: 'p1',
      scopeType: 'project',
    }))
    expect(client.from).toHaveBeenCalledWith('projects')
    expect(client.from).toHaveBeenCalledWith('project_stages')
    expect(document.body.textContent).toContain('banana-box://invite?token=card')
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
    from: vi.fn(() => ({
      upsert: vi.fn(async () => ({ data: [], error: null })),
    })),
  }
}

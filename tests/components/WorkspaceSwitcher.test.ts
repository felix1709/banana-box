import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import WorkspaceSwitcher from '@/components/workspaces/WorkspaceSwitcher.vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('WorkspaceSwitcher', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows local-only state when cloud is unavailable', () => {
    const auth = useAuthStore()
    auth.cloudAvailable = false

    const wrapper = mount(WorkspaceSwitcher)

    expect(wrapper.text()).toContain('本地离线模式')
  })

  it('shows active workspace for a logged-in user', () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    const workspaces = useWorkspacesStore()
    workspaces.profile = {
      id: 'user-1',
      email: 'correct@example.com',
      displayName: 'Correct',
      avatarUrl: null,
      createdAt: 'now',
      updatedAt: 'now',
    }
    workspaces.workspaces = [{
      id: 'workspace-1',
      name: '个人空间',
      ownerId: 'user-1',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-1'

    const wrapper = mount(WorkspaceSwitcher)

    expect(wrapper.text()).toContain('个人空间')
    expect(wrapper.text()).toContain('correct@example.com')
    expect(wrapper.text()).not.toContain('a@example.com')
  })

  it('shows and edits the collaboration nickname', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.client = {} as never
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    const workspaces = useWorkspacesStore()
    workspaces.profile = {
      id: 'user-1',
      email: '000001@banana-box.local',
      displayName: '导演',
      avatarUrl: null,
      createdAt: 'now',
      updatedAt: 'now',
    }
    workspaces.updateDisplayName = vi.fn(async () => {
      if (workspaces.profile) workspaces.profile.displayName = '制片'
    })
    workspaces.workspaces = [{
      id: 'workspace-1',
      name: '000001 的个人空间',
      ownerId: 'user-1',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-1'

    const wrapper = mount(WorkspaceSwitcher)
    expect(wrapper.text()).toContain('导演')

    await wrapper.get('[data-action="edit-display-name"]').trigger('click')
    await wrapper.get('[data-field="display-name"]').setValue('制片')
    await wrapper.get('[data-action="save-display-name"]').trigger('click')

    expect(workspaces.updateDisplayName).toHaveBeenCalledWith({}, '制片')
  })

  it('falls back to a readable workspace name when stored text is corrupted', () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    const workspaces = useWorkspacesStore()
    workspaces.workspaces = [{
      id: 'workspace-1',
      name: '000001 ?????',
      ownerId: 'user-1',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-1'

    const wrapper = mount(WorkspaceSwitcher)

    expect(wrapper.text()).toContain('000001 的个人空间')
    expect(wrapper.text()).not.toContain('?????')
  })

  it('signs out and clears workspace state', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    auth.signOut = vi.fn()
    const workspaces = useWorkspacesStore()
    workspaces.clear = vi.fn()

    const wrapper = mount(WorkspaceSwitcher)
    await wrapper.find('[data-action="auth-sign-out"]').trigger('click')

    expect(auth.signOut).toHaveBeenCalled()
    expect(workspaces.clear).toHaveBeenCalled()
  })
})

import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import InviteDialog from '@/components/collaboration/InviteDialog.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('InviteDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('searches users and sends a server-side project invitation', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = {} as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.searchInviteRecipients = vi.fn(async () => [
      { id: 'user-2', email: '000002@banana-box.local', displayName: '剪辑' },
      { id: 'user-3', email: '000003@banana-box.local', displayName: '剪辑助理' },
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

    const wrapper = mount(InviteDialog, { props: { projectId: 'p1', canInvite: true } })
    await wrapper.get('[data-field="invite-search"]').setValue('剪辑')
    await wrapper.get('[data-action="search-invite-users"]').trigger('click')
    await flushPromises()

    expect(members.searchInviteRecipients).toHaveBeenCalledWith({}, '剪辑', 'user-1')
    expect(wrapper.text()).toContain('剪辑')
    expect(wrapper.text()).toContain('剪辑助理')

    await wrapper.get('[data-action="add-invite-user"][data-user-id="user-2"]').trigger('click')
    await flushPromises()

    expect(members.createProjectUserInvite).toHaveBeenCalledWith({}, expect.objectContaining({
      workspaceId: 'workspace-1',
      projectId: 'p1',
      role: 'viewer',
      recipient: expect.objectContaining({ id: 'user-2' }),
      userId: 'user-1',
    }))
    expect(wrapper.find('.invite-link').exists()).toBe(false)
    expect(wrapper.text()).toContain('已发送')
  })

  it('does not search or invite when the selected project is not public', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = {} as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.searchInviteRecipients = vi.fn()
    members.createProjectUserInvite = vi.fn()

    const wrapper = mount(InviteDialog, { props: { projectId: 'p1', canInvite: false } })
    await wrapper.get('[data-field="invite-search"]').setValue('剪辑')
    await wrapper.get('[data-action="search-invite-users"]').trigger('click')

    expect(members.searchInviteRecipients).not.toHaveBeenCalled()
    expect(members.createProjectUserInvite).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('先设为公共项目')
  })
})

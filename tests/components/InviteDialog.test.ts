import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import InviteDialog from '@/components/collaboration/InviteDialog.vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'
import { useMembersStore } from '@/stores/members'

describe('InviteDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('creates and displays an invite link', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = {} as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const members = useMembersStore()
    members.createInvite = vi.fn(async () => ({
      id: 'invite-1',
      workspaceId: 'workspace-1',
      projectId: null,
      scopeType: 'workspace',
      role: 'viewer',
      email: null,
      expiresAt: 'tomorrow',
      url: 'banana-box://invite?token=abc',
    }))

    const wrapper = mount(InviteDialog, { props: { projectId: null } })
    await wrapper.get('[data-action="create-invite"]').trigger('click')

    expect(members.createInvite).toHaveBeenCalled()
    expect(wrapper.text()).toContain('banana-box://invite?token=abc')
  })
})

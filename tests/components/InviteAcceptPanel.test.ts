import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import InviteAcceptPanel from '@/components/collaboration/InviteAcceptPanel.vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'

describe('InviteAcceptPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('accepts a pasted invite token', async () => {
    const auth = useAuthStore()
    auth.client = {} as never
    auth.user = { id: 'user-1' } as never
    const members = useMembersStore()
    members.acceptInvite = vi.fn(async () => ({ workspaceId: 'workspace-1', projectId: null, role: 'viewer' }))

    const wrapper = mount(InviteAcceptPanel)
    await wrapper.get('input').setValue('token-123')
    await wrapper.get('[data-action="accept-invite"]').trigger('click')

    expect(members.acceptInvite).toHaveBeenCalledWith({}, 'token-123')
    expect(wrapper.text()).toContain('已加入')
  })
})

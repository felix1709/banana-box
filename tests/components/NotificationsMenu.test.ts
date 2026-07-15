import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import NotificationsMenu from '@/components/collaboration/NotificationsMenu.vue'
import { useAuthStore } from '@/stores/auth'
import { useNotificationsStore } from '@/stores/notifications'
import { useSyncStatusStore } from '@/stores/syncStatus'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('NotificationsMenu', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows project invite notifications and accepts them from the user menu', async () => {
    const auth = useAuthStore()
    auth.client = {} as never
    auth.user = { id: 'user-2', email: '000002@banana-box.local' } as never
    const notifications = useNotificationsStore()
    notifications.loadUnread = vi.fn(async () => undefined)
    notifications.notifications = [{
      id: 'notification-1',
      workspaceId: 'workspace-1',
      recipientUserId: 'user-2',
      actorUserId: 'user-1',
      kind: 'invite',
      targetType: 'project_invite',
      targetId: 'invite-1',
      readAt: null,
      createdAt: 'now',
    }]
    notifications.acceptInviteNotification = vi.fn(async () => ({
      workspaceId: 'workspace-1',
      projectId: 'project-1',
      role: 'editor',
    }))
    const workspaces = useWorkspacesStore()
    const sync = useSyncStatusStore()
    sync.pullWorkspace = vi.fn(async () => undefined)

    const wrapper = mount(NotificationsMenu)

    await wrapper.get('[data-action="notifications-menu"]').trigger('click')
    expect(wrapper.text()).toContain('项目邀请')

    await wrapper.get('[data-action="accept-project-invite"]').trigger('click')
    await flushPromises()

    expect(notifications.acceptInviteNotification).toHaveBeenCalledWith({}, 'notification-1', 'invite-1')
    expect(workspaces.activeWorkspaceId).toBe('workspace-1')
    expect(sync.pullWorkspace).toHaveBeenCalledWith({}, 'workspace-1')
  })
})

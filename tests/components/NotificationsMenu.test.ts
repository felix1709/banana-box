import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import NotificationsMenu from '@/components/collaboration/NotificationsMenu.vue'
import { STAGE_DEFINITIONS } from '@/domain/production'
import { useAuthStore } from '@/stores/auth'
import { useNotificationsStore } from '@/stores/notifications'
import { useScheduleRequestsStore } from '@/stores/scheduleRequests'
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

  it('lets the project owner approve a schedule change request from notifications', async () => {
    const auth = useAuthStore()
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
    const notifications = useNotificationsStore()
    notifications.loadUnread = vi.fn(async () => undefined)
    notifications.notifications = [{
      id: 'notification-2',
      workspaceId: 'workspace-1',
      recipientUserId: 'user-1',
      actorUserId: 'user-2',
      kind: 'project_update',
      targetType: 'project_schedule_request',
      targetId: 'request-1',
      readAt: null,
      createdAt: 'now',
    }]
    const requests = useScheduleRequestsStore()
    const scheduleRequest = {
      id: 'request-1',
      workspaceId: 'workspace-1',
      projectId: 'project-1',
      stageKey: STAGE_DEFINITIONS[0].key,
      requestedStartDate: '2026-07-03',
      requestedEndDate: '2026-07-12',
      reason: '分镜素材比预期晚两天到齐',
      status: 'pending',
      requestedBy: 'user-2',
      decidedBy: null,
      decisionNote: '',
      decidedAt: null,
      createdAt: 'now',
      updatedAt: 'now',
    }
    requests.loadRequest = vi.fn(async () => {
      requests.activeRequest = scheduleRequest
      return scheduleRequest
    })
    requests.approveRequest = vi.fn(async () => ({
      ...requests.activeRequest!,
      status: 'approved',
      decidedBy: 'user-1',
      decisionNote: '同意调整',
      decidedAt: 'now',
    }))
    const sync = useSyncStatusStore()
    sync.pullWorkspace = vi.fn(async () => undefined)

    const wrapper = mount(NotificationsMenu)

    await wrapper.get('[data-action="notifications-menu"]').trigger('click')
    await wrapper.get('[data-action="view-schedule-request"]').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('分镜素材比预期晚两天到齐')

    await wrapper.get('[data-field="schedule-decision-note"]').setValue('同意调整')
    await wrapper.get('[data-action="approve-schedule-request"]').trigger('click')
    await flushPromises()

    expect(requests.approveRequest).toHaveBeenCalledWith({}, expect.objectContaining({
      requestId: 'request-1',
      notificationId: 'notification-2',
      actorUserId: 'user-1',
      actorName: '导演',
      decisionNote: '同意调整',
    }))
    expect(sync.pullWorkspace).toHaveBeenCalledWith({}, 'workspace-1')
  })
})

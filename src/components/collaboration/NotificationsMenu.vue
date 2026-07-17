<script setup lang="ts">
import { ref } from 'vue'
import { Bell } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useNotificationsStore } from '@/stores/notifications'
import { useScheduleRequestsStore } from '@/stores/scheduleRequests'
import { useSyncStatusStore } from '@/stores/syncStatus'
import { useWorkspacesStore } from '@/stores/workspaces'

const auth = useAuthStore()
const notifications = useNotificationsStore()
const scheduleRequests = useScheduleRequestsStore()
const sync = useSyncStatusStore()
const workspaces = useWorkspacesStore()
const open = ref(false)
const status = ref('')
const error = ref('')
const activeScheduleNotificationId = ref('')
const decisionNote = ref('')

async function refresh() {
  if (!auth.client || !auth.user) return
  await notifications.loadUnread(auth.client, auth.user.id)
}

async function toggle() {
  open.value = !open.value
  if (open.value) await refresh()
}

async function acceptProjectInvite(notificationId: string, inviteId: string) {
  if (!auth.client) return
  status.value = ''
  error.value = ''
  try {
    const accepted = await notifications.acceptInviteNotification(auth.client, notificationId, inviteId)
    workspaces.addSharedWorkspace(accepted.workspaceId)
    await sync.pullWorkspace(auth.client, accepted.workspaceId)
    status.value = '已加入项目'
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  }
}

async function viewScheduleRequest(notificationId: string, requestId: string) {
  if (!auth.client) return
  status.value = ''
  error.value = ''
  activeScheduleNotificationId.value = notificationId
  decisionNote.value = ''
  try {
    await scheduleRequests.loadRequest(auth.client, requestId)
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  }
}

function actorName() {
  return workspaces.profile?.displayName || auth.user?.email?.split('@')[0] || '项目创建人'
}

async function decideScheduleRequest(decision: 'approve' | 'reject') {
  if (!auth.client || !auth.user || !scheduleRequests.activeRequest) return
  status.value = ''
  error.value = ''
  try {
    const input = {
      requestId: scheduleRequests.activeRequest.id,
      notificationId: activeScheduleNotificationId.value,
      actorUserId: auth.user.id,
      actorName: actorName(),
      decisionNote: decisionNote.value,
    }
    const result = decision === 'approve'
      ? await scheduleRequests.approveRequest(auth.client, input)
      : await scheduleRequests.rejectRequest(auth.client, input)
    await sync.pullWorkspace(auth.client, result.workspaceId)
    notifications.notifications = notifications.notifications.filter((item) => item.id !== activeScheduleNotificationId.value)
    activeScheduleNotificationId.value = ''
    decisionNote.value = ''
    status.value = decision === 'approve' ? '已同意调整' : '已拒绝调整'
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  }
}
</script>

<template>
  <div class="notifications-menu">
    <button
      data-action="notifications-menu"
      type="button"
      title="通知"
      :aria-expanded="open"
      @click="toggle"
    >
      <Bell
        :size="14"
        aria-hidden="true"
      />
      <span>{{ notifications.unreadCount }}</span>
    </button>
    <div
      v-if="open"
      class="notifications-popover"
    >
      <p v-if="notifications.unreadCount === 0">
        暂无通知
      </p>
      <article
        v-for="notification in notifications.notifications"
        :key="notification.id"
        class="notification-item"
      >
        <strong>{{ notification.kind === 'invite' ? '项目邀请' : '通知' }}</strong>
        <span>{{ notification.targetType === 'project_invite' ? '邀请你加入一个公共项目' : notification.targetType }}</span>
        <button
          v-if="notification.kind === 'invite' && notification.targetType === 'project_invite'"
          data-action="accept-project-invite"
          type="button"
          @click="acceptProjectInvite(notification.id, notification.targetId)"
        >
          接受
        </button>
        <button
          v-if="notification.targetType === 'project_schedule_request'"
          data-action="view-schedule-request"
          type="button"
          @click="viewScheduleRequest(notification.id, notification.targetId)"
        >
          查看
        </button>
      </article>
      <section
        v-if="scheduleRequests.activeRequest && activeScheduleNotificationId"
        class="schedule-request-review"
      >
        <strong>时间调整申请</strong>
        <span>
          {{ scheduleRequests.activeRequest.stageKey }}
          {{ scheduleRequests.activeRequest.requestedStartDate }}
          -
          {{ scheduleRequests.activeRequest.requestedEndDate }}
        </span>
        <p>{{ scheduleRequests.activeRequest.reason }}</p>
        <textarea
          v-model="decisionNote"
          data-field="schedule-decision-note"
          rows="2"
          placeholder="批复说明"
        />
        <div>
          <button
            data-action="approve-schedule-request"
            type="button"
            @click="decideScheduleRequest('approve')"
          >
            同意
          </button>
          <button
            data-action="reject-schedule-request"
            type="button"
            @click="decideScheduleRequest('reject')"
          >
            拒绝
          </button>
        </div>
      </section>
      <p
        v-if="status"
        class="notification-status"
      >
        {{ status }}
      </p>
      <p
        v-if="error"
        class="notification-error"
        role="alert"
      >
        {{ error }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.notifications-menu {
  position: relative;
  display: inline-flex;
}

.notifications-menu > button {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  gap: 5px;
  padding: 0 8px;
}

.notifications-popover {
  position: absolute;
  z-index: 2;
  top: calc(100% + 6px);
  right: 0;
  display: grid;
  width: 230px;
  gap: 7px;
  padding: 8px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-sm);
  background: rgba(5, 14, 22, 0.98);
  box-shadow: var(--bb-shadow-floating);
}

.notifications-popover p {
  margin: 0;
  color: var(--bb-text-soft);
  font-size: 11px;
}

.notification-item {
  display: grid;
  gap: 5px;
  min-width: 0;
  padding: 7px;
  border: 1px solid rgba(123, 255, 226, 0.12);
  border-radius: var(--bb-radius-xs);
  background: rgba(123, 255, 226, 0.06);
}

.notification-item strong {
  font-size: 12px;
}

.notification-item span {
  color: var(--bb-text-soft);
  font-size: 11px;
}

.notification-item button {
  min-height: 26px;
  justify-self: start;
  padding: 0 8px;
}

.schedule-request-review {
  display: grid;
  gap: 6px;
  min-width: 0;
  padding: 8px;
  border: 1px solid rgba(244, 196, 48, 0.32);
  border-radius: var(--bb-radius-xs);
  background: rgba(244, 196, 48, 0.08);
}

.schedule-request-review strong {
  font-size: 12px;
}

.schedule-request-review span,
.schedule-request-review p {
  margin: 0;
  color: var(--bb-text-soft);
  font-size: 11px;
}

.schedule-request-review textarea {
  width: 100%;
  min-width: 0;
  padding: 6px;
  resize: vertical;
}

.schedule-request-review div {
  display: flex;
  gap: 6px;
}

.schedule-request-review button {
  min-height: 26px;
  padding: 0 8px;
}

.notification-status {
  color: var(--bb-primary-strong) !important;
}

.notification-error {
  color: #ffb6c0 !important;
}
</style>

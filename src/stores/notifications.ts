import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import type { AppNotification, NotificationKind } from '@/types'

type NotificationsClient = Pick<SupabaseClient, 'from'>

interface NotificationRow {
  id: string
  workspace_id: string
  recipient_user_id: string
  actor_user_id: string | null
  kind: NotificationKind
  target_type: string
  target_id: string
  read_at: string | null
  created_at: string
}

function mapNotification(row: NotificationRow): AppNotification {
  return {
    id: row.id,
    workspaceId: row.workspace_id,
    recipientUserId: row.recipient_user_id,
    actorUserId: row.actor_user_id,
    kind: row.kind,
    targetType: row.target_type,
    targetId: row.target_id,
    readAt: row.read_at,
    createdAt: row.created_at,
  }
}

export const useNotificationsStore = defineStore('notifications', {
  state: () => ({
    notifications: [] as AppNotification[],
    loading: false,
    error: '',
  }),
  getters: {
    unreadCount(state) {
      return state.notifications.filter((notification) => !notification.readAt).length
    },
  },
  actions: {
    async loadUnread(client: NotificationsClient, userId: string) {
      this.loading = true
      this.error = ''
      const response = await client
        .from('notifications')
        .select('*')
        .eq('recipient_user_id', userId)
        .is('read_at', null)
        .order('created_at', { ascending: false })

      if (response.error) this.error = response.error.message
      else this.notifications = ((response.data ?? []) as NotificationRow[]).map(mapNotification)
      this.loading = false
    },
  },
})

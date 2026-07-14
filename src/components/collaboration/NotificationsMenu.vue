<script setup lang="ts">
import { Bell } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useNotificationsStore } from '@/stores/notifications'

const auth = useAuthStore()
const notifications = useNotificationsStore()

async function refresh() {
  if (!auth.client || !auth.user) return
  await notifications.loadUnread(auth.client, auth.user.id)
}
</script>

<template>
  <button
    class="notifications-menu"
    type="button"
    title="通知"
    @click="refresh"
  >
    <Bell
      :size="14"
      aria-hidden="true"
    />
    <span>{{ notifications.unreadCount }}</span>
  </button>
</template>

<style scoped>
.notifications-menu {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  gap: 5px;
  padding: 0 8px;
}
</style>

<script setup lang="ts">
import { computed } from 'vue'
import { Cloud, RefreshCw, TriangleAlert } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useSyncStatusStore } from '@/stores/syncStatus'
import { useWorkspacesStore } from '@/stores/workspaces'

const auth = useAuthStore()
const sync = useSyncStatusStore()
const workspaces = useWorkspacesStore()

const label = computed(() => {
  if (sync.state === 'syncing') return '同步中'
  if (sync.state === 'error') return '同步失败'
  if (sync.state === 'conflict') return `冲突 ${sync.conflicts.length}`
  if (!auth.user) return '本地'
  if (sync.lastSyncedAt) return '已同步'
  return '云端待同步'
})

async function refresh() {
  if (!auth.client || !workspaces.activeWorkspaceId) return
  await sync.pullWorkspace(auth.client, workspaces.activeWorkspaceId)
}
</script>

<template>
  <button
    class="sync-status"
    type="button"
    :class="[`sync-status-${sync.state}`]"
    :title="sync.error || label"
    @click="refresh"
  >
    <TriangleAlert
      v-if="sync.state === 'error' || sync.state === 'conflict'"
      :size="14"
      aria-hidden="true"
    />
    <RefreshCw
      v-else-if="sync.state === 'syncing'"
      :size="14"
      aria-hidden="true"
    />
    <Cloud
      v-else
      :size="14"
      aria-hidden="true"
    />
    <span>{{ label }}</span>
  </button>
</template>

<style scoped>
.sync-status {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  gap: 6px;
  padding: 0 9px;
  font-size: 12px;
}

.sync-status-error,
.sync-status-conflict {
  border-color: var(--bb-danger-border);
  color: #ffb6c0;
}
</style>

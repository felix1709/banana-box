import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'

type SyncClient = Pick<SupabaseClient, 'from'>
type SyncState = 'idle' | 'syncing' | 'synced' | 'error' | 'conflict'

const SYNC_TABLES = [
  'prompt_categories',
  'prompts',
  'projects',
  'project_stages',
  'daily_task_days',
  'daily_task_groups',
  'daily_tasks',
  'comments',
  'project_activity_log',
] as const

export const useSyncStatusStore = defineStore('syncStatus', {
  state: () => ({
    state: 'idle' as SyncState,
    lastSyncedAt: '',
    error: '',
    pendingOutbox: 0,
    conflicts: [] as string[],
    snapshots: {} as Record<string, unknown[]>,
  }),
  actions: {
    async pullWorkspace(client: SyncClient, workspaceId: string) {
      this.state = 'syncing'
      this.error = ''

      for (const table of SYNC_TABLES) {
        const response = await client
          .from(table)
          .select('*')
          .eq('workspace_id', workspaceId)

        if (response.error) {
          this.state = 'error'
          this.error = response.error.message
          return
        }
        this.snapshots[table] = response.data ?? []
      }

      this.lastSyncedAt = new Date().toISOString()
      this.state = this.conflicts.length > 0 ? 'conflict' : 'synced'
    },
    markConflict(recordId: string) {
      if (!this.conflicts.includes(recordId)) this.conflicts.push(recordId)
      this.state = 'conflict'
    },
    clearError() {
      this.error = ''
      if (this.state === 'error') this.state = 'idle'
    },
  },
})

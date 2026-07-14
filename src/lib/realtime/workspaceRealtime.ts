import type { RealtimeChannel, SupabaseClient } from '@supabase/supabase-js'

type RealtimeClient = Pick<SupabaseClient, 'channel'>

const REALTIME_TABLES = ['projects', 'daily_tasks', 'comments', 'notifications'] as const

export interface WorkspaceRealtimeSubscription {
  channel: RealtimeChannel
  unsubscribe: () => void
}

export function subscribeWorkspaceRealtime(
  client: RealtimeClient,
  workspaceId: string,
  onChange: () => void,
): WorkspaceRealtimeSubscription {
  const channel = client.channel(`workspace:${workspaceId}`)

  for (const table of REALTIME_TABLES) {
    channel.on(
      'postgres_changes',
      {
        event: '*',
        schema: 'public',
        table,
        filter: `workspace_id=eq.${workspaceId}`,
      },
      onChange,
    )
  }

  channel.subscribe()

  return {
    channel,
    unsubscribe: () => {
      channel.unsubscribe()
    },
  }
}

import { defineStore } from 'pinia'
import type { RealtimeChannel, SupabaseClient } from '@supabase/supabase-js'
import type { PresenceUser } from '@/types'

type PresenceClient = Pick<SupabaseClient, 'channel'>

interface JoinWorkspaceInput {
  workspaceId: string
  userId: string
  email: string
}

export const usePresenceStore = defineStore('presence', {
  state: () => ({
    channel: null as RealtimeChannel | null,
    onlineUsers: [] as PresenceUser[],
    currentWorkspaceId: '',
  }),
  actions: {
    async joinWorkspace(client: PresenceClient, input: JoinWorkspaceInput) {
      await this.leave()
      const channel = client.channel(`presence:${input.workspaceId}`)
      this.currentWorkspaceId = input.workspaceId
      this.channel = channel

      channel
        .on('presence', { event: 'sync' }, () => {
          const state = channel.presenceState<PresenceUser>()
          this.applyPresenceState(state)
        })
        .subscribe(async (status) => {
          if (status === 'SUBSCRIBED') {
            await channel.track({
              userId: input.userId,
              email: input.email,
              onlineAt: new Date().toISOString(),
            })
          }
        })
    },
    applyPresenceState(state: Record<string, PresenceUser[]>) {
      this.onlineUsers = Object.values(state)
        .flat()
        .filter((user, index, all) => all.findIndex((item) => item.userId === user.userId) === index)
    },
    async leave() {
      if (this.channel) await this.channel.unsubscribe()
      this.channel = null
      this.onlineUsers = []
      this.currentWorkspaceId = ''
    },
  },
})

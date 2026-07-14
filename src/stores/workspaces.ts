import { defineStore } from 'pinia'
import type { SupabaseClient, User } from '@supabase/supabase-js'
import type { AppProfile, Workspace } from '@/types'

type WorkspaceSupabaseClient = Pick<SupabaseClient, 'from' | 'rpc'>

interface ProfileRow {
  id: string
  email: string
  display_name: string
  avatar_url: string | null
  created_at: string
  updated_at: string
}

interface WorkspaceRow {
  id: string
  name: string
  owner_id: string
  created_at: string
  updated_at: string
}

interface WorkspaceMembershipRow {
  role: string
  workspaces: WorkspaceRow | WorkspaceRow[] | null
}

interface WorkspaceBootstrapResult {
  profile: ProfileRow
  workspace: WorkspaceRow
}

function displayNameFromEmail(email: string) {
  return email.split('@')[0] || 'Banana Box User'
}

function mapProfile(row: ProfileRow): AppProfile {
  return {
    id: row.id,
    email: row.email,
    displayName: row.display_name,
    avatarUrl: row.avatar_url,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

function mapWorkspace(row: WorkspaceRow): Workspace {
  return {
    id: row.id,
    name: row.name,
    ownerId: row.owner_id,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

function workspaceFromMembership(row: WorkspaceMembershipRow) {
  if (!row.workspaces) return null
  return Array.isArray(row.workspaces) ? row.workspaces[0] : row.workspaces
}

export const useWorkspacesStore = defineStore('workspaces', {
  state: () => ({
    profile: null as AppProfile | null,
    workspaces: [] as Workspace[],
    activeWorkspaceId: '',
    loading: false,
    error: '',
  }),
  getters: {
    activeWorkspace(state): Workspace | null {
      return state.workspaces.find((workspace) => workspace.id === state.activeWorkspaceId) ?? null
    },
  },
  actions: {
    async bootstrapForUser(client: WorkspaceSupabaseClient, user: User) {
      this.loading = true
      this.error = ''

      try {
        const email = user.email ?? ''
        const displayName = displayNameFromEmail(email)
        const workspaceName = `${displayName} 的个人空间`

        const bootstrapResponse = await client.rpc('bootstrap_user_workspace', {
          workspace_name: workspaceName,
          user_email: email,
          user_display_name: displayName,
        })

        if (bootstrapResponse.error) {
          this.error = bootstrapResponse.error.message
          return
        }

        const bootstrap = bootstrapResponse.data as WorkspaceBootstrapResult
        this.profile = mapProfile(bootstrap.profile)
        this.workspaces = [mapWorkspace(bootstrap.workspace)]
        this.activeWorkspaceId = bootstrap.workspace.id
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.loading = false
      }
    },
    async loadMembershipsOrCreateDefault(
      client: WorkspaceSupabaseClient,
      userId: string,
      displayName: string,
    ) {
      const membershipsResponse = await client
        .from('workspace_members')
        .select('role, workspaces(id, name, owner_id, created_at, updated_at)')
        .eq('user_id', userId)
        .order('created_at')

      if (membershipsResponse.error) {
        this.error = membershipsResponse.error.message
        return
      }

      const existing = ((membershipsResponse.data ?? []) as WorkspaceMembershipRow[])
        .map(workspaceFromMembership)
        .filter((workspace): workspace is WorkspaceRow => Boolean(workspace))
        .map(mapWorkspace)

      if (existing.length > 0) {
        this.workspaces = existing
        this.activeWorkspaceId = this.activeWorkspaceId || existing[0].id
        return
      }

      const workspaceName = `${displayName} 的个人空间`
      const workspaceResponse = await client.rpc('ensure_personal_workspace', {
        workspace_name: workspaceName,
      })

      if (workspaceResponse.error) {
        this.error = workspaceResponse.error.message
        return
      }

      const workspace = mapWorkspace(workspaceResponse.data as WorkspaceRow)
      this.workspaces = [workspace]
      this.activeWorkspaceId = workspace.id
    },
    setActiveWorkspace(workspaceId: string) {
      if (this.workspaces.some((workspace) => workspace.id === workspaceId)) {
        this.activeWorkspaceId = workspaceId
      }
    },
    addSharedWorkspace(workspaceId: string) {
      if (!this.workspaces.some((workspace) => workspace.id === workspaceId)) {
        const now = new Date().toISOString()
        this.workspaces.push({
          id: workspaceId,
          name: '协作项目空间',
          ownerId: '',
          createdAt: now,
          updatedAt: now,
        })
      }
      this.activeWorkspaceId = workspaceId
    },
    async updateDisplayName(client: WorkspaceSupabaseClient, displayName: string) {
      const normalized = displayName.trim()
      if (!this.profile || !normalized) return
      this.error = ''
      const response = await client
        .from('profiles')
        .update({ display_name: normalized })
        .eq('id', this.profile.id)
        .single()
      if (response.error) {
        this.error = response.error.message
        return
      }
      this.profile = mapProfile(response.data as ProfileRow)
    },
    clear() {
      this.profile = null
      this.workspaces = []
      this.activeWorkspaceId = ''
      this.error = ''
      this.loading = false
    },
  },
})

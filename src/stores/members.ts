import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import type { CollaborationInvite, InviteScopeType, WorkspaceMember, WorkspaceRole } from '@/types'

type MemberClient = Pick<SupabaseClient, 'from'>
type InviteAcceptClient = Pick<SupabaseClient, 'rpc'>
type InviteRole = Exclude<WorkspaceRole, 'owner'>

interface CreateInviteInput {
  appOrigin: string
  workspaceId: string
  projectId: string | null
  scopeType: InviteScopeType
  role: InviteRole
  email?: string | null
  userId: string
  token?: string
}

interface MemberRow {
  workspace_id: string
  user_id: string
  role: WorkspaceRole
  created_at: string
}

interface ProfileLookupRow {
  id: string
  email: string
  display_name: string
}

interface InviteRecipient {
  id: string
  email: string
  displayName: string
}

async function tokenHash(token: string) {
  if (typeof crypto !== 'undefined' && crypto.subtle) {
    const bytes = new TextEncoder().encode(token)
    const digest = await crypto.subtle.digest('SHA-256', bytes)
    return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
  }
  return btoa(token).replace(/=+$/g, '')
}

function daysFromNow(days: number) {
  return new Date(Date.now() + days * 24 * 60 * 60 * 1000).toISOString()
}

function randomToken() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function mapMember(row: MemberRow): WorkspaceMember {
  return {
    workspaceId: row.workspace_id,
    userId: row.user_id,
    role: row.role,
    createdAt: row.created_at,
  }
}

function emailFromInviteIdentity(identity: string) {
  const trimmed = identity.trim()
  if (/^\d{6}$/.test(trimmed)) return `${trimmed}@banana-box.local`
  return trimmed.includes('@') ? trimmed : ''
}

function escapeSupabaseOrValue(value: string) {
  return value.replace(/\\/g, '\\\\').replace(/,/g, '\\,')
}

export const useMembersStore = defineStore('members', {
  state: () => ({
    members: [] as WorkspaceMember[],
    invites: [] as CollaborationInvite[],
    loading: false,
    error: '',
  }),
  actions: {
    async loadMembers(client: MemberClient, workspaceId: string) {
      this.loading = true
      this.error = ''
      const response = await client
        .from('workspace_members')
        .select('workspace_id, user_id, role, created_at')
        .eq('workspace_id', workspaceId)
        .order('created_at')

      if (response.error) this.error = response.error.message
      else this.members = ((response.data ?? []) as MemberRow[]).map(mapMember)
      this.loading = false
    },
    async createInvite(client: MemberClient, input: CreateInviteInput) {
      const token = input.token ?? randomToken()
      const expiresAt = daysFromNow(7)
      const response = await client
        .from('invites')
        .insert({
          workspace_id: input.workspaceId,
          project_id: input.projectId,
          scope_type: input.scopeType,
          role: input.role,
          email: input.email ?? null,
          token_hash: await tokenHash(token),
          expires_at: expiresAt,
          created_by: input.userId,
          updated_by: input.userId,
        })
        .select('id')
        .single()

      if (response.error) throw new Error(response.error.message)

      const invite: CollaborationInvite = {
        id: String(response.data.id),
        workspaceId: input.workspaceId,
        projectId: input.projectId,
        scopeType: input.scopeType,
        role: input.role,
        email: input.email ?? null,
        expiresAt,
        url: `${input.appOrigin}?token=${encodeURIComponent(token)}&workspace=${input.workspaceId}`,
      }
      this.invites.unshift(invite)
      return invite
    },
    async resolveInviteRecipient(client: MemberClient, identity: string): Promise<InviteRecipient> {
      const trimmed = identity.trim()
      if (!trimmed) throw new Error('请输入邀请ID或昵称')

      const accountEmail = emailFromInviteIdentity(trimmed)
      const filters = accountEmail
        ? `email.eq.${escapeSupabaseOrValue(accountEmail)},display_name.eq.${escapeSupabaseOrValue(trimmed)}`
        : `display_name.eq.${escapeSupabaseOrValue(trimmed)}`
      const response = await client
        .from('profiles')
        .select('id, email, display_name')
        .or(filters)
        .limit(2)

      if (response.error) throw new Error(response.error.message)
      const rows = (response.data ?? []) as ProfileLookupRow[]
      if (rows.length === 0) throw new Error('未找到该用户')
      if (rows.length > 1) throw new Error('找到多个同名用户，请改用 000002 这类账号ID')

      return {
        id: rows[0].id,
        email: rows[0].email,
        displayName: rows[0].display_name,
      }
    },
    async acceptInvite(client: InviteAcceptClient, token: string) {
      const response = await client.rpc('accept_invite', { invite_token: token.trim() })
      if (response.error) throw new Error(response.error.message)
      const data = response.data as { workspace_id: string; project_id: string | null; role: InviteRole }
      return {
        workspaceId: data.workspace_id,
        projectId: data.project_id,
        role: data.role,
      }
    },
  },
})

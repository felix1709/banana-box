import type { WorkspaceRole } from './workspace'

export type InviteScopeType = 'workspace' | 'project'
export type CommentTargetType = 'workspace' | 'project' | 'project_task' | 'daily_task'
export type NotificationKind = 'comment' | 'mention' | 'project_update' | 'invite'

export interface CollaborationInvite {
  id: string
  workspaceId: string
  projectId: string | null
  scopeType: InviteScopeType
  role: Exclude<WorkspaceRole, 'owner'>
  email: string | null
  url: string
  expiresAt: string
}

export interface CommentItem {
  id: string
  workspaceId: string
  targetType: CommentTargetType
  targetId: string
  parentCommentId: string | null
  body: string
  createdBy: string
  updatedBy: string
  revision: number
  deletedAt: string | null
  createdAt: string
  updatedAt: string
}

export interface AppNotification {
  id: string
  workspaceId: string
  recipientUserId: string
  actorUserId: string | null
  kind: NotificationKind
  targetType: string
  targetId: string
  readAt: string | null
  createdAt: string
}

export interface PresenceUser {
  userId: string
  email: string
  onlineAt?: string
}

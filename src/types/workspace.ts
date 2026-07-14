export type WorkspaceRole = 'owner' | 'editor' | 'commenter' | 'viewer'

export interface Workspace {
  id: string
  name: string
  ownerId: string
  createdAt: string
  updatedAt: string
}

export interface WorkspaceMember {
  workspaceId: string
  userId: string
  role: WorkspaceRole
  createdAt: string
}

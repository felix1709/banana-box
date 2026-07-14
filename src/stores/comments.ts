import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import type { CommentItem, CommentTargetType } from '@/types'

type CommentsClient = Pick<SupabaseClient, 'from'>

interface CommentRow {
  id: string
  workspace_id: string
  target_type: CommentTargetType
  target_id: string
  parent_comment_id: string | null
  body: string
  created_by: string
  updated_by: string
  revision: number
  deleted_at: string | null
  created_at: string
  updated_at: string
}

interface AddCommentInput {
  workspaceId: string
  targetType: CommentTargetType
  targetId: string
  parentCommentId: string | null
  body: string
  createdBy: string
  mentionedUserIds?: string[]
}

function mapComment(row: CommentRow): CommentItem {
  return {
    id: row.id,
    workspaceId: row.workspace_id,
    targetType: row.target_type,
    targetId: row.target_id,
    parentCommentId: row.parent_comment_id,
    body: row.body,
    createdBy: row.created_by,
    updatedBy: row.updated_by,
    revision: row.revision,
    deletedAt: row.deleted_at,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

export const useCommentsStore = defineStore('comments', {
  state: () => ({
    comments: [] as CommentItem[],
    loading: false,
    error: '',
  }),
  getters: {
    topLevelComments(state) {
      return state.comments.filter((comment) => !comment.parentCommentId)
    },
  },
  actions: {
    async loadForTarget(
      client: CommentsClient,
      workspaceId: string,
      targetType: CommentTargetType,
      targetId: string,
    ) {
      this.loading = true
      this.error = ''
      const response = await client
        .from('comments')
        .select('*')
        .eq('workspace_id', workspaceId)
        .eq('target_type', targetType)
        .eq('target_id', targetId)
        .order('created_at')

      if (response.error) this.error = response.error.message
      else this.comments = ((response.data ?? []) as CommentRow[]).map(mapComment)
      this.loading = false
    },
    async addComment(client: CommentsClient, input: AddCommentInput) {
      const response = await client
        .from('comments')
        .insert({
          workspace_id: input.workspaceId,
          target_type: input.targetType,
          target_id: input.targetId,
          parent_comment_id: input.parentCommentId,
          body: input.body,
          created_by: input.createdBy,
          updated_by: input.createdBy,
        })
        .select('id')
        .single()

      if (response.error) throw new Error(response.error.message)

      const commentId = String(response.data.id)
      const mentionedUserIds = input.mentionedUserIds ?? []
      if (mentionedUserIds.length > 0) {
        await client.from('comment_mentions').insert(
          mentionedUserIds.map((userId) => ({
            comment_id: commentId,
            mentioned_user_id: userId,
          })),
        )
        await client.from('notifications').insert(
          mentionedUserIds.map((userId) => ({
            workspace_id: input.workspaceId,
            recipient_user_id: userId,
            actor_user_id: input.createdBy,
            kind: 'mention',
            target_type: input.targetType,
            target_id: input.targetId,
            created_by: input.createdBy,
            updated_by: input.createdBy,
          })),
        )
      }

      return commentId
    },
  },
})

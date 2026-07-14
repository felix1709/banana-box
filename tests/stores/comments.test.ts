import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useCommentsStore } from '@/stores/comments'

describe('comments store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('loads comment threads for a project target', async () => {
    const store = useCommentsStore()
    const client = clientMock([{
      id: 'comment-1',
      workspace_id: 'workspace-1',
      target_type: 'project',
      target_id: 'project-1',
      parent_comment_id: null,
      body: '请看这里',
      created_by: 'user-1',
      updated_by: 'user-1',
      revision: 0,
      deleted_at: null,
      created_at: 'now',
      updated_at: 'now',
    }])

    await store.loadForTarget(client as never, 'workspace-1', 'project', 'project-1')

    expect(store.comments).toHaveLength(1)
    expect(store.comments[0].body).toBe('请看这里')
  })

  it('creates a reply and mention notifications from @member ids', async () => {
    const store = useCommentsStore()
    const client = clientMock([])

    await store.addComment(client as never, {
      workspaceId: 'workspace-1',
      targetType: 'project',
      targetId: 'project-1',
      parentCommentId: 'comment-parent',
      body: '@user-2 看一下',
      createdBy: 'user-1',
      mentionedUserIds: ['user-2'],
    })

    expect(client.from).toHaveBeenCalledWith('comments')
    expect(client.from).toHaveBeenCalledWith('comment_mentions')
    expect(client.from).toHaveBeenCalledWith('notifications')
  })
})

function clientMock(rows: unknown[]) {
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          eq: vi.fn(() => ({
            eq: vi.fn(() => ({
              order: vi.fn(async () => ({ data: rows, error: null })),
            })),
          })),
        })),
      })),
      insert: vi.fn(() => ({
        select: vi.fn(() => ({
          single: vi.fn(async () => ({ data: { id: 'comment-1' }, error: null })),
        })),
      })),
    })),
  }
}

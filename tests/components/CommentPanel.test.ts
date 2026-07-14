import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CommentPanel from '@/components/collaboration/CommentPanel.vue'
import { useAuthStore } from '@/stores/auth'
import { useCommentsStore } from '@/stores/comments'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('CommentPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('loads and posts project comments', async () => {
    const auth = useAuthStore()
    auth.client = {} as never
    auth.user = { id: 'user-1' } as never
    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    const comments = useCommentsStore()
    comments.loadForTarget = vi.fn()
    comments.addComment = vi.fn()

    const wrapper = mount(CommentPanel, {
      props: { targetType: 'project', targetId: 'project-1' },
    })

    expect(comments.loadForTarget).toHaveBeenCalledWith({}, 'workspace-1', 'project', 'project-1')
    await wrapper.get('textarea').setValue('新的留言')
    await wrapper.get('[data-action="post-comment"]').trigger('click')

    expect(comments.addComment).toHaveBeenCalled()
  })
})

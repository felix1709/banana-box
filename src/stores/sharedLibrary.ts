import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import type { Prompt, SharedPrompt } from '@/types'
import { copyToClipboard } from '@/lib/ipc'

type SharedLibraryClient = Pick<SupabaseClient, 'from'>
type UploadResult = { status: 'uploaded'; prompt: SharedPrompt } | { status: 'duplicate' }

interface SharedPromptRow {
  id: string
  title: string
  content: string
  tags: string[] | null
  image_ref: string | null
  created_by: string
  created_by_name: string | null
  created_at: string
  updated_at: string
}

function normalizeTitleKey(title: string) {
  return title.trim().replace(/\s+/g, ' ').toLocaleLowerCase()
}

function rowToSharedPrompt(row: SharedPromptRow): SharedPrompt {
  return {
    id: row.id,
    title: row.title,
    content: row.content,
    tags: row.tags ?? [],
    image: row.image_ref,
    createdBy: row.created_by,
    createdByName: row.created_by_name ?? '',
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

export const useSharedLibraryStore = defineStore('sharedLibrary', {
  state: () => ({
    prompts: [] as SharedPrompt[],
    search: '',
    loading: false,
    error: '',
  }),

  getters: {
    sortedPrompts(state): SharedPrompt[] {
      return [...state.prompts].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
    },
    filteredPrompts(): SharedPrompt[] {
      const keyword = this.search.trim().toLocaleLowerCase()
      if (!keyword) return this.sortedPrompts
      return this.sortedPrompts.filter((prompt) =>
        prompt.title.toLocaleLowerCase().includes(keyword) ||
        prompt.content.toLocaleLowerCase().includes(keyword) ||
        prompt.tags.some((tag) => tag.toLocaleLowerCase().includes(keyword)),
      )
    },
  },

  actions: {
    hydrate(prompts: SharedPrompt[]) {
      this.prompts = prompts
    },

    async load() {
      const auth = useAuthStore()
      if (!auth.client || !auth.user) {
        this.error = '请先登录后再打开共享库'
        return
      }
      await this.loadWithClient(auth.client)
    },

    async loadWithClient(client: SharedLibraryClient) {
      this.loading = true
      this.error = ''
      try {
        const response = await client
          .from('shared_prompts')
          .select('id,title,content,tags,image_ref,created_by,created_by_name,created_at,updated_at')
          .is('deleted_at', null)
          .order('updated_at', { ascending: false })
        if (response.error) throw new Error(response.error.message)
        this.prompts = ((response.data ?? []) as SharedPromptRow[]).map(rowToSharedPrompt)
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.loading = false
      }
    },

    async uploadLocalPrompt(prompt: Prompt, titleOverride?: string): Promise<UploadResult> {
      const auth = useAuthStore()
      if (!auth.client || !auth.user) throw new Error('请先登录后再上传共享库')
      const title = (titleOverride ?? prompt.title).trim()
      if (!title) throw new Error('提示词标题不能为空')
      const titleKey = normalizeTitleKey(title)

      const duplicateResponse = await auth.client
        .from('shared_prompts')
        .select('id')
        .eq('title_key', titleKey)
        .is('deleted_at', null)
        .maybeSingle()
      if (duplicateResponse.error) throw new Error(duplicateResponse.error.message)
      if (duplicateResponse.data) return { status: 'duplicate' }

      const row = {
        title,
        title_key: titleKey,
        content: prompt.content,
        tags: prompt.tags,
        image_ref: prompt.image,
        created_by: auth.user.id,
        updated_by: auth.user.id,
        created_by_name: auth.user.email ?? '',
      }
      const response = await auth.client
        .from('shared_prompts')
        .insert(row)
        .select('id,title,content,tags,image_ref,created_by,created_by_name,created_at,updated_at')
        .single()
      if (response.error) throw new Error(response.error.message)
      const sharedPrompt = rowToSharedPrompt(response.data as SharedPromptRow)
      this.prompts = [sharedPrompt, ...this.prompts.filter((item) => item.id !== sharedPrompt.id)]
      return { status: 'uploaded', prompt: sharedPrompt }
    },

    downloadToLocal(sharedPrompt: SharedPrompt) {
      const localPrompt = useLibraryStore().addSharedPromptReference(sharedPrompt)
      const auth = useAuthStore()
      if (auth.client && auth.user) {
        void auth.client.from('user_prompt_refs').upsert({
          user_id: auth.user.id,
          shared_prompt_id: sharedPrompt.id,
          local_prompt_id: localPrompt.id,
        })
      }
      return localPrompt
    },

    hasLocalReference(sharedPromptId: string) {
      return useLibraryStore().library.prompts.some((prompt) => prompt.sharedPromptId === sharedPromptId)
    },

    async copySharedPrompt(promptId: string) {
      const prompt = this.prompts.find((item) => item.id === promptId)
      if (!prompt) return
      await copyToClipboard(prompt.content)
    },

    async deleteSharedPrompt(promptId: string) {
      const auth = useAuthStore()
      if (!auth.client || !auth.user) throw new Error('请先登录后再删除共享提示词')
      if (!auth.isCloudAdmin) throw new Error('只有管理员可以删除共享提示词')

      const response = await auth.client
        .from('shared_prompts')
        .update({
          deleted_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          updated_by: auth.user.id,
        })
        .eq('id', promptId)
      if (response.error) throw new Error(response.error.message)
      this.prompts = this.prompts.filter((prompt) => prompt.id !== promptId)
    },
  },
})

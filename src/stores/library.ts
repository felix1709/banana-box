// src/stores/library.ts
// 数据 store：增删改查、搜索、分类筛选。IPC 调用抽到 lib/ipc.ts。

import { defineStore } from 'pinia'
import { v4 as uuid } from 'uuid'
import type { Library, Prompt, Category } from '@/types'
import * as ipc from '@/lib/ipc'

export const FAVORITES_CATEGORY_ID = '__favorites__'
export const DEFAULT_API_BASE_URL = 'https://ai.leihuo.netease.com'
export const DEFAULT_REVERSE_MODEL = 'doubao-seed-1-6-vision-250815'
export const DEFAULT_REVERSE_MODELS = [
  'doubao-seed-1-6-vision-250815',
  'gpt-5.4-mini',
  'qwen3.5-omni-plus',
  'qwen3-vl-plus',
]

function normalizeSettings(settings: Partial<Library['settings']>): Library['settings'] {
  return {
    hotkey: settings.hotkey ?? 'Ctrl+Shift+B',
    theme: settings.theme ?? 'auto',
    apiBaseUrl: settings.apiBaseUrl ?? DEFAULT_API_BASE_URL,
    apiKey: settings.apiKey ?? '',
    reverseModel: settings.reverseModel ?? DEFAULT_REVERSE_MODEL,
    availableReverseModels: settings.availableReverseModels?.length
      ? settings.availableReverseModels
      : DEFAULT_REVERSE_MODELS,
  }
}

function normalizeLibrary(library: Library): Library {
  return {
    ...library,
    prompts: library.prompts.map((prompt, index) => ({
      ...prompt,
      favorite: prompt.favorite ?? false,
      order: prompt.order ?? index,
    })),
    settings: normalizeSettings(library.settings),
  }
}

const emptyLibrary = (): Library => ({
  version: 1,
  categories: [],
  prompts: [],
  settings: normalizeSettings({}),
})

export const useLibraryStore = defineStore('library', {
  state: () => ({
    library: emptyLibrary() as Library,
    search: '' as string,
    currentCategoryId: null as string | null, // null = 全部
    loaded: false as boolean,
  }),
  getters: {
    filteredPrompts(state): Prompt[] {
      const kw = state.search.trim().toLowerCase()
      return state.library.prompts
        .filter((p) => {
          if (state.currentCategoryId === FAVORITES_CATEGORY_ID && !p.favorite) return false
          if (
            state.currentCategoryId &&
            state.currentCategoryId !== FAVORITES_CATEGORY_ID &&
            p.categoryId !== state.currentCategoryId
          ) {
            return false
          }
          if (!kw) return true
          return (
            p.title.toLowerCase().includes(kw) ||
            p.content.toLowerCase().includes(kw) ||
            p.tags.some((t) => t.toLowerCase().includes(kw))
          )
        })
        .sort((a, b) => a.order - b.order)
    },
    categories(state): Category[] {
      return [...state.library.categories].sort((a, b) => a.order - b.order)
    },
  },
  actions: {
    // 测试用：直接注入数据，不调 IPC
    hydrate(lib: Library) {
      this.library = normalizeLibrary(lib)
      this.loaded = true
    },
    async load() {
      this.library = normalizeLibrary(await ipc.loadLibrary())
      this.loaded = true
    },
    async persist() {
      await ipc.saveLibrary(this.library)
    },
    addPrompt(input: Omit<Prompt, 'id' | 'createdAt' | 'updatedAt' | 'favorite' | 'order'>) {
      const now = Math.floor(Date.now() / 1000)
      const nextOrder =
        this.library.prompts.length === 0
          ? 0
          : Math.max(...this.library.prompts.map((prompt) => prompt.order)) + 1
      const p: Prompt = {
        ...input,
        id: uuid(),
        favorite: false,
        order: nextOrder,
        createdAt: now,
        updatedAt: now,
      }
      this.library.prompts.push(p)
      this.persist()
    },
    updatePrompt(id: string, patch: Partial<Prompt>) {
      const p = this.library.prompts.find((x) => x.id === id)
      if (!p) return
      Object.assign(p, patch, { updatedAt: Math.floor(Date.now() / 1000) })
      this.persist()
    },
    async deletePrompt(id: string) {
      const p = this.library.prompts.find((x) => x.id === id)
      if (p?.image) await ipc.deleteImage(p.image)
      this.library.prompts = this.library.prompts.filter((x) => x.id !== id)
      this.persist()
    },
    addCategory(name: string, color = '#888') {
      const order = this.library.categories.length
      this.library.categories.push({ id: uuid(), name, color, order })
      this.persist()
    },
    deleteCategory(id: string) {
      this.library.categories = this.library.categories.filter((c) => c.id !== id)
      for (const p of this.library.prompts) {
        if (p.categoryId === id) p.categoryId = null
      }
      this.persist()
    },
    async copyPrompt(id: string) {
      const p = this.library.prompts.find((x) => x.id === id)
      if (!p) return
      await ipc.copyToClipboard(p.content)
    },
    toggleFavorite(id: string) {
      const p = this.library.prompts.find((x) => x.id === id)
      if (!p) return
      p.favorite = !p.favorite
      p.updatedAt = Math.floor(Date.now() / 1000)
      this.persist()
    },
    movePromptBefore(draggedId: string, targetId: string) {
      if (draggedId === targetId) return
      const ordered = [...this.library.prompts].sort((a, b) => a.order - b.order)
      const draggedIndex = ordered.findIndex((p) => p.id === draggedId)
      if (draggedIndex < 0) return
      const [dragged] = ordered.splice(draggedIndex, 1)
      const targetIndex = ordered.findIndex((p) => p.id === targetId)
      if (targetIndex < 0) return
      ordered.splice(targetIndex, 0, dragged)
      ordered.forEach((p, index) => {
        p.order = index
      })
      this.persist()
    },
  },
})

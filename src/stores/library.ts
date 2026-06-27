// src/stores/library.ts
// 数据 store：增删改查、搜索、分类筛选。IPC 调用抽到 lib/ipc.ts。

import { defineStore } from 'pinia'
import { v4 as uuid } from 'uuid'
import type { Library, Prompt, Category } from '@/types'
import * as ipc from '@/lib/ipc'

const emptyLibrary = (): Library => ({
  version: 1,
  categories: [],
  prompts: [],
  settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
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
      return state.library.prompts.filter((p) => {
        if (state.currentCategoryId && p.categoryId !== state.currentCategoryId) return false
        if (!kw) return true
        return (
          p.title.toLowerCase().includes(kw) ||
          p.content.toLowerCase().includes(kw) ||
          p.tags.some((t) => t.toLowerCase().includes(kw))
        )
      })
    },
    categories(state): Category[] {
      return [...state.library.categories].sort((a, b) => a.order - b.order)
    },
  },
  actions: {
    // 测试用：直接注入数据，不调 IPC
    hydrate(lib: Library) {
      this.library = lib
      this.loaded = true
    },
    async load() {
      this.library = await ipc.loadLibrary()
      this.loaded = true
    },
    async persist() {
      await ipc.saveLibrary(this.library)
    },
    addPrompt(input: Omit<Prompt, 'id' | 'createdAt' | 'updatedAt'>) {
      const now = Math.floor(Date.now() / 1000)
      const p: Prompt = { ...input, id: uuid(), createdAt: now, updatedAt: now }
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
  },
})

import { defineStore } from 'pinia'
import type { AtlasCategory, AtlasEntry } from '@/types/atlas'
import * as atlasIpc from '@/lib/atlas-ipc'

export const useAtlasStore = defineStore('atlas', {
  state: () => ({
    entries: [] as AtlasEntry[],
    categories: [] as AtlasCategory[],
    selectedId: null as string | null,
    selectedCategoryId: null as string | null,
    search: '' as string,
    loaded: false,
  }),
  getters: {
    filteredEntries(state): AtlasEntry[] {
      const keyword = state.search.trim().toLowerCase()
      return state.entries
        .filter(
          (entry) =>
            !state.selectedCategoryId ||
            entry.categoryIds.includes(state.selectedCategoryId),
        )
        .filter(
          (entry) =>
            !keyword ||
            `${entry.title} ${entry.tags.join(' ')}`.toLowerCase().includes(keyword),
        )
        .sort((a, b) => b.score - a.score)
    },
    selectedEntry(state): AtlasEntry | null {
      return state.entries.find((entry) => entry.id === state.selectedId) ?? null
    },
    selectedCategory(state): AtlasCategory | null {
      return (
        state.categories.find((category) => category.id === state.selectedCategoryId) ?? null
      )
    },
  },
  actions: {
    hydrate(entries: AtlasEntry[], categories: AtlasCategory[] = []) {
      this.entries = entries
      this.categories = categories
      this.loaded = true
    },
    async load() {
      const [entries, categories] = await Promise.all([
        atlasIpc.loadAtlasEntries(),
        atlasIpc.loadAtlasCategories(),
      ])
      this.entries = entries
      this.categories = categories
      this.loaded = true
    },
    select(id: string) {
      this.selectedId = id
    },
    selectCategory(id: string | null) {
      this.selectedCategoryId = id
    },
    async createCategory(name: string) {
      this.categories = await atlasIpc.createAtlasCategory(name)
    },
    async renameCategory(id: string, name: string) {
      this.categories = await atlasIpc.renameAtlasCategory(id, name)
    },
    async deleteCategory(id: string) {
      this.categories = await atlasIpc.deleteAtlasCategory(id)
      if (this.selectedCategoryId === id) {
        this.selectedCategoryId = null
      }
      for (const entry of this.entries) {
        entry.categoryIds = entry.categoryIds.filter((categoryId) => categoryId !== id)
      }
    },
    async setEntryCategories(entryId: string, categoryIds: string[]) {
      this.categories = await atlasIpc.setAtlasEntryCategories(entryId, categoryIds)
      const entry = this.entries.find((item) => item.id === entryId)
      if (entry) {
        entry.categoryIds = [...categoryIds]
      }
    },
    async deleteEntry(entryId: string) {
      await atlasIpc.deleteAtlasEntry(entryId)
      this.entries = this.entries.filter((entry) => entry.id !== entryId)
      for (const category of this.categories) {
        category.entryIds = category.entryIds.filter((id) => id !== entryId)
      }
    },
  },
})

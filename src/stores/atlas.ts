import { defineStore } from 'pinia'
import type { AtlasEntry } from '@/types/atlas'
import * as atlasIpc from '@/lib/atlas-ipc'

export const useAtlasStore = defineStore('atlas', {
  state: () => ({
    entries: [] as AtlasEntry[],
    selectedId: null as string | null,
    search: '' as string,
    dimension: null as string | null,
    loaded: false,
  }),
  getters: {
    filteredEntries(state): AtlasEntry[] {
      const keyword = state.search.trim().toLowerCase()
      return state.entries
        .filter((entry) => !state.dimension || entry.dimension === state.dimension)
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
  },
  actions: {
    hydrate(entries: AtlasEntry[]) {
      this.entries = entries
      this.loaded = true
    },
    async load() {
      this.entries = await atlasIpc.loadAtlasEntries()
      this.loaded = true
    },
    select(id: string) {
      this.selectedId = id
    },
  },
})

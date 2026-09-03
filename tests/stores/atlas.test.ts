import { describe, it, expect } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAtlasStore } from '@/stores/atlas'

describe('atlas store', () => {
  it('hydrates entries', () => {
    setActivePinia(createPinia())
    const store = useAtlasStore()
    store.hydrate([
      {
        id: 'a',
        title: '测试',
        dimension: 'lighting-atmosphere',
        tags: [],
        image: '',
        score: 1,
        status: 'confirmed',
        file: 'entries/a.md',
      },
    ])
    expect(store.filteredEntries).toHaveLength(1)
  })
})

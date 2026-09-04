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
        categoryIds: [],
      },
    ])
    expect(store.filteredEntries).toHaveLength(1)
  })

  it('filters entries by the selected custom category', () => {
    setActivePinia(createPinia())
    const store = useAtlasStore()
    store.hydrate(
      [
        {
          id: 'a',
          title: '项目A参考',
          dimension: 'style',
          tags: [],
          image: '',
          score: 1,
          status: 'confirmed',
          file: 'entries/a.md',
          categoryIds: ['project-a'],
        },
        {
          id: 'b',
          title: '普通参考',
          dimension: 'style',
          tags: [],
          image: '',
          score: 1,
          status: 'confirmed',
          file: 'entries/b.md',
          categoryIds: [],
        },
      ],
      [
        {
          id: 'project-a',
          name: '项目A',
          entryIds: ['a'],
          createdAt: '2026-09-03T00:00:00+08:00',
        },
      ],
    )

    store.selectCategory('project-a')
    expect(store.filteredEntries.map((entry) => entry.id)).toEqual(['a'])

    store.selectCategory(null)
    expect(store.filteredEntries.map((entry) => entry.id)).toEqual(['a', 'b'])
  })
})

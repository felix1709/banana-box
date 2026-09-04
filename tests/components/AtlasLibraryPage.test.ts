import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import AtlasLibraryPage from '@/components/atlas/AtlasLibraryPage.vue'
import { useAtlasStore } from '@/stores/atlas'

vi.mock('@/lib/atlas-ipc', () => ({
  getAtlasServiceStatus: vi.fn().mockResolvedValue(false),
  loadAtlasEntries: vi.fn().mockResolvedValue([
    {
      id: 'a',
      title: '测试参考',
      dimension: 'lighting-atmosphere',
      tags: ['霓虹'],
      image: 'C:/atlas/assets/a.webp',
      score: 1,
      status: 'confirmed',
      file: 'entries/a.md',
      categoryIds: [],
    },
  ]),
  loadAtlasCategories: vi.fn().mockResolvedValue([]),
  loadAtlasEntry: vi.fn().mockResolvedValue('---\nid: a\n---\n\n## 提示词片段\n逆光测试'),
  loadAtlasImage: vi.fn().mockResolvedValue('blob:atlas'),
  openAtlasFolder: vi.fn(),
  startAtlasService: vi.fn(),
  stopAtlasService: vi.fn(),
}))

describe('AtlasLibraryPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders hydrated atlas entries', async () => {
    const store = useAtlasStore()
    store.hydrate([
      {
        id: 'a',
        title: '测试参考',
        dimension: 'lighting-atmosphere',
        tags: ['霓虹'],
        image: 'C:/atlas/assets/a.webp',
        score: 1,
        status: 'confirmed',
        file: 'entries/a.md',
        categoryIds: [],
      },
    ])

    const wrapper = mount(AtlasLibraryPage, {
      global: { plugins: [createPinia()] },
    })
    await flushPromises()

    expect(wrapper.find('.atlas-page').exists()).toBe(true)
    expect(wrapper.find('.atlas-grid').exists()).toBe(true)
    expect(wrapper.find('.atlas-card-favorite').exists()).toBe(true)
  })
})

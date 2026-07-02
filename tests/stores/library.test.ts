// tests/stores/library.test.ts
import { setActivePinia, createPinia } from 'pinia'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { FAVORITES_CATEGORY_ID, useLibraryStore } from '@/stores/library'
import type { Library } from '@/types'

// mock 掉 IPC 层，store 在 jsdom 里不触发真实 Tauri 调用
vi.mock('@/lib/ipc', () => ({
  loadLibrary: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  saveImage: vi.fn(),
  deleteImage: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
  exportLibrary: vi.fn(),
  importLibrary: vi.fn(),
}))

function seed(): Library {
  return {
    version: 1,
    categories: [
      { id: 'c1', name: '写作', color: '#f59e0b', order: 0 },
      { id: 'c2', name: '翻译', color: '#3b82f6', order: 1 },
    ],
    prompts: [
      {
        id: 'p1',
        title: '总结助手',
        content: '请总结三点',
        categoryId: 'c1',
        tags: ['中文'],
        image: null,
        createdAt: 1,
        updatedAt: 1,
      },
      {
        id: 'p2',
        title: '润色',
        content: 'polish this',
        categoryId: 'c2',
        tags: ['en'],
        image: null,
        createdAt: 2,
        updatedAt: 2,
      },
    ],
    settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
  }
}

describe('library store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    const s = useLibraryStore()
    s.hydrate(seed())
  })

  it('filters by search term across title/content/tags', () => {
    const s = useLibraryStore()
    s.search = '总结'
    expect(s.filteredPrompts.map((p) => p.id)).toEqual(['p1'])
    s.search = 'en'
    expect(s.filteredPrompts.map((p) => p.id)).toEqual(['p2'])
    s.search = ''
    expect(s.filteredPrompts.length).toBe(2)
  })

  it('filters by current category', () => {
    const s = useLibraryStore()
    s.currentCategoryId = 'c1'
    expect(s.filteredPrompts.map((p) => p.id)).toEqual(['p1'])
  })

  it('normalizes old prompts with favorite and order defaults', () => {
    const s = useLibraryStore()

    expect(s.library.prompts.map((p) => ({ id: p.id, favorite: p.favorite, order: p.order }))).toEqual([
      { id: 'p1', favorite: false, order: 0 },
      { id: 'p2', favorite: false, order: 1 },
    ])
  })

  it('filters favorite prompts through the fixed favorites category', () => {
    const s = useLibraryStore()
    s.toggleFavorite('p2')
    s.currentCategoryId = FAVORITES_CATEGORY_ID

    expect(s.filteredPrompts.map((p) => p.id)).toEqual(['p2'])
  })

  it('sorts prompts by order and can move one prompt before another', () => {
    const s = useLibraryStore()
    s.updatePrompt('p1', { order: 10 })
    s.updatePrompt('p2', { order: 20 })

    s.movePromptBefore('p2', 'p1')

    expect(s.filteredPrompts.map((p) => p.id)).toEqual(['p2', 'p1'])
    expect(s.library.prompts.find((p) => p.id === 'p2')?.order).toBeLessThan(
      s.library.prompts.find((p) => p.id === 'p1')?.order ?? 0,
    )
  })

  it('addPrompt appends with uuid and timestamps', () => {
    const s = useLibraryStore()
    const before = s.library.prompts.length
    s.addPrompt({ title: '新', content: 'x', categoryId: null, tags: [], image: null })
    expect(s.library.prompts.length).toBe(before + 1)
    expect(s.library.prompts[before].id.length).toBeGreaterThan(8)
    expect(s.library.prompts[before].createdAt).toBeGreaterThan(0)
    expect(s.library.prompts[before].favorite).toBe(false)
    expect(s.library.prompts[before].order).toBe(before)
  })

  it('deletePrompt removes prompt', () => {
    const s = useLibraryStore()
    s.deletePrompt('p1')
    expect(s.library.prompts.find((p) => p.id === 'p1')).toBeUndefined()
  })

  it('updatePrompt mutates and bumps updatedAt', () => {
    const s = useLibraryStore()
    s.updatePrompt('p1', { title: '改后' })
    expect(s.library.prompts[0].title).toBe('改后')
    expect(s.library.prompts[0].updatedAt).toBeGreaterThanOrEqual(1)
  })

  it('deleteCategory nulls categoryId of its prompts', () => {
    const s = useLibraryStore()
    s.deleteCategory('c1')
    expect(s.library.categories.find((c) => c.id === 'c1')).toBeUndefined()
    expect(s.library.prompts[0].categoryId).toBeNull()
  })

  it('hydrates old libraries with default reverse API settings', () => {
    const s = useLibraryStore()

    expect((s.library.settings as any).apiBaseUrl).toBe('https://ai.leihuo.netease.com')
    expect((s.library.settings as any).apiKey).toBe('')
    expect((s.library.settings as any).reverseModel).toBe('doubao-seed-1-6-vision-250815')
    expect((s.library.settings as any).availableReverseModels).toEqual([
      'doubao-seed-1-6-vision-250815',
      'gpt-5.4-mini',
      'qwen3.5-omni-plus',
      'qwen3-vl-plus',
    ])
  })
})

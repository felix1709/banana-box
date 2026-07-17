import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SharedLibraryPage from '@/components/shared/SharedLibraryPage.vue'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'

vi.mock('@/lib/ipc', () => ({
  loadLibrary: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  saveImage: vi.fn(),
  deleteImage: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
  exportLibrary: vi.fn(),
}))

describe('SharedLibraryPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
  })

  it('loads shared prompts and lets users double-click to use one', async () => {
    const auth = useAuthStore()
    const shared = useSharedLibraryStore()
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = clientMock() as never
    const copy = vi.spyOn(shared, 'copySharedPrompt')

    const wrapper = mount(SharedLibraryPage)
    await flushPromises()

    expect(wrapper.find('[data-shared-prompt-card="shared-1"]').exists()).toBe(true)

    await wrapper.get('[data-shared-prompt-card="shared-1"]').trigger('dblclick')

    expect(copy).toHaveBeenCalledWith('shared-1')
  })

  it('downloads a shared prompt into the local library as a reference', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = clientMock() as never
    const wrapper = mount(SharedLibraryPage)
    await flushPromises()

    await wrapper.get('[data-shared-prompt-card="shared-1"]').trigger('click')
    await wrapper.get('[data-action="download-shared-prompt"][data-shared-prompt-id="shared-1"]').trigger('click')

    expect(useLibraryStore().library.prompts).toHaveLength(1)
    expect(useLibraryStore().library.prompts[0].sharedPromptId).toBe('shared-1')
  })

  it('filters shared prompts from the page search field', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = clientMock([{
      id: 'shared-1',
      title: '日式王家卫',
      content: '低饱和电影感',
      tags: ['cinema'],
      image_ref: null,
      created_by: 'user-2',
      created_by_name: '剪辑师',
      created_at: '2026-07-17T00:00:00Z',
      updated_at: '2026-07-17T00:00:00Z',
    }, {
      id: 'shared-2',
      title: '产品摄影',
      content: 'clean product lighting',
      tags: ['photo'],
      image_ref: null,
      created_by: 'user-2',
      created_by_name: '剪辑师',
      created_at: '2026-07-17T00:00:00Z',
      updated_at: '2026-07-17T00:00:00Z',
    }]) as never
    const wrapper = mount(SharedLibraryPage)
    await flushPromises()

    await wrapper.get('[data-field="shared-library-search"]').setValue('photo')

    expect(wrapper.find('[data-shared-prompt-card="shared-1"]').exists()).toBe(false)
    expect(wrapper.find('[data-shared-prompt-card="shared-2"]').exists()).toBe(true)
  })

  it('marks shared prompts already downloaded into the local library', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = clientMock() as never
    useLibraryStore().addSharedPromptReference({
      id: 'shared-1',
      title: 'Shared Prompt',
      content: 'Use this prompt',
      tags: ['shared'],
      image: null,
      createdBy: 'user-2',
      createdByName: '剪辑师',
      createdAt: '2026-07-17T00:00:00Z',
      updatedAt: '2026-07-17T00:00:00Z',
    })
    const wrapper = mount(SharedLibraryPage)
    await flushPromises()

    await wrapper.get('[data-shared-prompt-card="shared-1"]').trigger('click')
    const downloadButton = wrapper.get('[data-action="download-shared-prompt"][data-shared-prompt-id="shared-1"]')
    expect(downloadButton.attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-shared-prompt-card="shared-1"]').text()).toContain('已下载')
  })
})

function clientMock(rows = [{
  id: 'shared-1',
  title: 'Shared Prompt',
  content: 'Use this prompt',
  tags: ['shared'],
  image_ref: null,
  created_by: 'user-2',
  created_by_name: '剪辑师',
  created_at: '2026-07-17T00:00:00Z',
  updated_at: '2026-07-17T00:00:00Z',
}]) {
  return {
    from: vi.fn(() => ({
      upsert: vi.fn(async () => ({ data: [], error: null })),
      select: vi.fn(() => ({
        is: vi.fn(() => ({
          order: vi.fn(async () => ({
            data: rows,
            error: null,
          })),
        })),
      })),
    })),
  }
}

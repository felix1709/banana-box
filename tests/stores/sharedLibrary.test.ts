import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuthStore } from '@/stores/auth'
import { useLibraryStore } from '@/stores/library'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'
import type { Prompt, SharedPrompt } from '@/types'
import { copyToClipboard } from '@/lib/ipc'

vi.mock('@/lib/ipc', () => ({
  loadLibrary: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  saveImage: vi.fn(),
  deleteImage: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
  exportLibrary: vi.fn(),
}))

describe('shared library store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('uploads a local prompt only when the shared title is unique', async () => {
    const auth = useAuthStore()
    const client = sharedClientMock({ duplicate: false })
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = client as never
    const shared = useSharedLibraryStore()

    const result = await shared.uploadLocalPrompt(localPrompt())

    expect(result.status).toBe('uploaded')
    expect(client.from).toHaveBeenCalledWith('shared_prompts')
    expect(client.insertedRows[0]).toMatchObject({
      title: 'Prompt A',
      title_key: 'prompt a',
      content: 'Shared content',
      created_by: 'user-1',
      updated_by: 'user-1',
    })
  })

  it('returns duplicate status instead of uploading when a shared title already exists', async () => {
    const auth = useAuthStore()
    const client = sharedClientMock({ duplicate: true })
    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    auth.client = client as never
    const shared = useSharedLibraryStore()

    const result = await shared.uploadLocalPrompt(localPrompt())

    expect(result).toEqual({ status: 'duplicate' })
    expect(client.insertedRows).toHaveLength(0)
  })

  it('downloads a shared prompt as a local reference without creating a duplicate local copy', async () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [],
      prompts: [],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    const shared = useSharedLibraryStore()

    shared.downloadToLocal(sharedPrompt())
    shared.downloadToLocal(sharedPrompt())

    expect(lib.library.prompts).toHaveLength(1)
    expect(lib.library.prompts[0]).toMatchObject({
      title: 'Shared Prompt',
      content: 'Use this prompt',
      sharedPromptId: 'shared-1',
      sourceType: 'shared',
    })
  })

  it('copies shared prompt content directly for double-click usage', async () => {
    const shared = useSharedLibraryStore()
    shared.hydrate([sharedPrompt()])

    await shared.copySharedPrompt('shared-1')

    expect(copyToClipboard).toHaveBeenCalledWith('Use this prompt')
  })

  it('filters shared prompts by title, content, and tags', () => {
    const shared = useSharedLibraryStore()
    shared.hydrate([
      sharedPrompt({ id: 'shared-1', title: '日式王家卫', content: '低饱和电影感', tags: ['cinema'] }),
      sharedPrompt({ id: 'shared-2', title: '产品摄影', content: 'clean product lighting', tags: ['photo'] }),
    ])

    shared.search = 'photo'
    expect(shared.filteredPrompts.map((prompt) => prompt.id)).toEqual(['shared-2'])

    shared.search = '王家卫'
    expect(shared.filteredPrompts.map((prompt) => prompt.id)).toEqual(['shared-1'])

    shared.search = '低饱和'
    expect(shared.filteredPrompts.map((prompt) => prompt.id)).toEqual(['shared-1'])
  })

  it('detects whether a shared prompt is already referenced locally', () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [],
      prompts: [],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    const shared = useSharedLibraryStore()

    expect(shared.hasLocalReference('shared-1')).toBe(false)

    shared.downloadToLocal(sharedPrompt())

    expect(shared.hasLocalReference('shared-1')).toBe(true)
  })
})

function localPrompt(overrides: Partial<Prompt> = {}): Prompt {
  return {
    id: 'prompt-1',
    title: 'Prompt A',
    content: 'Shared content',
    categoryId: null,
    tags: ['tag'],
    image: null,
    favorite: false,
    order: 0,
    createdAt: 1,
    updatedAt: 2,
    ...overrides,
  }
}

function sharedPrompt(overrides: Partial<SharedPrompt> = {}): SharedPrompt {
  return {
    id: 'shared-1',
    title: 'Shared Prompt',
    content: 'Use this prompt',
    tags: ['shared'],
    image: null,
    createdBy: 'user-1',
    createdByName: 'Felix',
    createdAt: '2026-07-17T00:00:00Z',
    updatedAt: '2026-07-17T00:00:00Z',
    ...overrides,
  }
}

function sharedClientMock(options: { duplicate: boolean }) {
  const insertedRows: unknown[] = []
  return {
    insertedRows,
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          is: vi.fn(() => ({
            maybeSingle: vi.fn(async () => ({
              data: options.duplicate ? { id: 'existing' } : null,
              error: null,
            })),
          })),
        })),
      })),
      insert: vi.fn((row: unknown) => {
        insertedRows.push(row)
        return {
          select: vi.fn(() => ({
            single: vi.fn(async () => ({
              data: { id: 'shared-1', ...row },
              error: null,
            })),
          })),
        }
      }),
    })),
  }
}

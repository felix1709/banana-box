import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useCloudSessionStore } from '@/stores/cloudSession'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

vi.mock('@/lib/ipc', () => ({
  loadCloudConfig: vi.fn(),
  saveCloudConfig: vi.fn(),
}))

describe('cloud session store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('loads local-only state when no cloud config exists', async () => {
    vi.mocked(loadCloudConfig).mockResolvedValue({
      supabaseUrl: '',
      hasAnonKey: false,
      cloudEnabled: false,
      updatedAt: null,
    })
    const store = useCloudSessionStore()

    await store.load()

    expect(store.readiness).toBe('local_only')
    expect(store.config?.cloudEnabled).toBe(false)
  })

  it('saves valid cloud config and marks the app configured', async () => {
    vi.mocked(saveCloudConfig).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    })
    const store = useCloudSessionStore()

    await store.save({
      supabaseUrl: 'https://example.supabase.co/',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(saveCloudConfig).toHaveBeenCalledWith({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })
    expect(store.readiness).toBe('configured')
    expect(store.error).toBe('')
  })

  it('keeps invalid config local and does not call the backend', async () => {
    const store = useCloudSessionStore()

    await store.save({
      supabaseUrl: 'http://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(saveCloudConfig).not.toHaveBeenCalled()
    expect(store.readiness).toBe('invalid')
    expect(store.error).toBe('URL_INSECURE')
    expect(store.config).toEqual({
      supabaseUrl: 'http://example.supabase.co',
      hasAnonKey: false,
      cloudEnabled: true,
      updatedAt: null,
    })
  })

  it('saves without a new anon key when a stored key already exists', async () => {
    vi.mocked(saveCloudConfig).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: false,
      updatedAt: '2026-07-13T00:00:00Z',
    })
    const store = useCloudSessionStore()
    store.config = {
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    }

    await store.save({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: false,
    })

    expect(saveCloudConfig).toHaveBeenCalledWith({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: false,
    })
    expect(store.readiness).toBe('local_only')
  })
})

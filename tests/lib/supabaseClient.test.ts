import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearSupabaseClientForTests,
  createSupabaseClientFromRuntimeConfig,
  getSupabaseClient,
} from '@/lib/supabaseClient'
import { loadCloudRuntimeConfig } from '@/lib/ipc'

vi.mock('@/lib/ipc', () => ({
  loadCloudRuntimeConfig: vi.fn(),
}))

vi.mock('@supabase/supabase-js', () => ({
  createClient: vi.fn((url: string, key: string) => ({
    __url: url,
    __key: key,
    auth: {},
    from: vi.fn(),
  })),
}))

describe('supabase client factory', () => {
  beforeEach(() => {
    clearSupabaseClientForTests()
    vi.clearAllMocks()
  })

  it('returns null when cloud runtime config is disabled', async () => {
    vi.mocked(loadCloudRuntimeConfig).mockResolvedValue({
      supabaseUrl: '',
      anonKey: '',
      cloudEnabled: false,
    })

    await expect(getSupabaseClient()).resolves.toBeNull()
  })

  it('creates a client when runtime config is enabled', async () => {
    vi.mocked(loadCloudRuntimeConfig).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    const client = await getSupabaseClient()

    expect(client).not.toBeNull()
  })

  it('rejects incomplete runtime config', () => {
    expect(() => createSupabaseClientFromRuntimeConfig({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: true,
    })).toThrow('CLOUD_RUNTIME_CONFIG_INCOMPLETE')
  })
})

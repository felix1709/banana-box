import { invoke } from '@tauri-apps/api/core'
import { describe, expect, it, vi } from 'vitest'
import {
  cloudReadiness,
  normalizeCloudConfigInput,
  validateCloudConfigInput,
} from '@/lib/cloud-config'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('cloud config validation', () => {
  it('normalizes a valid Supabase URL without logging or changing the anon key', () => {
    const normalized = normalizeCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co/',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(normalized).toEqual({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })
    expect(validateCloudConfigInput(normalized)).toEqual({ ok: true, code: 'OK' })
    expect(cloudReadiness({
      supabaseUrl: normalized.supabaseUrl,
      hasAnonKey: true,
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    })).toBe('configured')
  })

  it('allows localhost http for local Supabase development', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'http://localhost:54321',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: true, code: 'OK' })
  })

  it('rejects insecure remote http URLs', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'http://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_INSECURE' })
  })

  it('rejects blank and malformed inputs with stable codes', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: '',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_REQUIRED' })
    expect(validateCloudConfigInput({
      supabaseUrl: 'not a url',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_INVALID' })
    expect(validateCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'ANON_KEY_REQUIRED' })
  })

  it('blocks service role keys from being stored in the desktop app', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'service_role.secret.must.not.ship',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'SERVICE_ROLE_KEY_BLOCKED' })
  })

  it('reports local only when cloud is disabled', () => {
    expect(cloudReadiness({
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: false,
      updatedAt: '2026-07-13T00:00:00Z',
    })).toBe('local_only')
  })
})

describe('cloud config IPC', () => {
  it('loads cloud config through an empty Tauri command payload', async () => {
    vi.mocked(invoke).mockResolvedValue({
      supabaseUrl: '',
      hasAnonKey: false,
      cloudEnabled: false,
      updatedAt: null,
    })

    await loadCloudConfig()

    expect(invoke).toHaveBeenCalledWith('load_cloud_config', {})
  })

  it('saves cloud config through a single input payload', async () => {
    vi.mocked(invoke).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    })

    await saveCloudConfig({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(invoke).toHaveBeenCalledWith('save_cloud_config', {
      input: {
        supabaseUrl: 'https://example.supabase.co',
        anonKey: 'anon-test-key',
        cloudEnabled: true,
      },
    })
  })
})

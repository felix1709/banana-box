import { createClient, type SupabaseClient } from '@supabase/supabase-js'
import { loadCloudRuntimeConfig } from '@/lib/ipc'
import type { CloudRuntimeConfig } from '@/types'

let cachedClient: SupabaseClient | null = null
let cachedSignature = ''

function runtimeSignature(config: CloudRuntimeConfig) {
  return `${config.supabaseUrl}|${config.anonKey.slice(0, 8)}|${config.cloudEnabled}`
}

export function createSupabaseClientFromRuntimeConfig(config: CloudRuntimeConfig): SupabaseClient {
  if (!config.cloudEnabled || !config.supabaseUrl || !config.anonKey) {
    throw new Error('CLOUD_RUNTIME_CONFIG_INCOMPLETE')
  }

  return createClient(config.supabaseUrl, config.anonKey, {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: false,
    },
  })
}

export async function getSupabaseClient(): Promise<SupabaseClient | null> {
  const config = await loadCloudRuntimeConfig()
  if (!config.cloudEnabled || !config.supabaseUrl || !config.anonKey) return null

  const signature = runtimeSignature(config)
  if (!cachedClient || cachedSignature !== signature) {
    cachedClient = createSupabaseClientFromRuntimeConfig(config)
    cachedSignature = signature
  }

  return cachedClient
}

export function clearSupabaseClientForTests() {
  cachedClient = null
  cachedSignature = ''
}

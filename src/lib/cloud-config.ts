import type {
  CloudConfig,
  CloudConfigValidationResult,
  CloudReadiness,
  SaveCloudConfigInput,
} from '@/types/cloud'

interface CloudConfigValidationOptions {
  hasExistingAnonKey?: boolean
}

function trimTrailingSlashes(value: string) {
  return value.trim().replace(/\/+$/, '')
}

function isLoopbackHttp(url: URL) {
  return (
    url.protocol === 'http:'
    && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
  )
}

export function normalizeCloudConfigInput(input: SaveCloudConfigInput): SaveCloudConfigInput {
  return {
    supabaseUrl: trimTrailingSlashes(input.supabaseUrl),
    anonKey: input.anonKey.trim(),
    cloudEnabled: input.cloudEnabled,
  }
}

export function validateCloudConfigInput(
  input: SaveCloudConfigInput,
  options: CloudConfigValidationOptions = {},
): CloudConfigValidationResult {
  const normalized = normalizeCloudConfigInput(input)
  if (!normalized.supabaseUrl) return { ok: false, code: 'URL_REQUIRED' }

  let parsed: URL
  try {
    parsed = new URL(normalized.supabaseUrl)
  } catch {
    return { ok: false, code: 'URL_INVALID' }
  }

  if (parsed.protocol !== 'https:' && !isLoopbackHttp(parsed)) {
    return { ok: false, code: 'URL_INSECURE' }
  }
  if (!normalized.anonKey && !options.hasExistingAnonKey) return { ok: false, code: 'ANON_KEY_REQUIRED' }
  if (normalized.anonKey.toLocaleLowerCase().includes('service_role')) {
    return { ok: false, code: 'SERVICE_ROLE_KEY_BLOCKED' }
  }

  return { ok: true, code: 'OK' }
}

export function cloudReadiness(config: CloudConfig | null): CloudReadiness {
  if (!config?.cloudEnabled) return 'local_only'
  return validateCloudConfigInput({
    supabaseUrl: config.supabaseUrl,
    anonKey: config.hasAnonKey ? 'stored-anon-key' : '',
    cloudEnabled: config.cloudEnabled,
  }).ok ? 'configured' : 'invalid'
}

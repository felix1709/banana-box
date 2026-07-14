export interface CloudConfig {
  supabaseUrl: string
  hasAnonKey: boolean
  cloudEnabled: boolean
  updatedAt: string | null
}

export interface CloudRuntimeConfig {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
}

export interface SaveCloudConfigInput {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
}

export type CloudReadiness = 'local_only' | 'configured' | 'invalid'

export interface CloudConfigValidationResult {
  ok: boolean
  code: 'OK' | 'URL_REQUIRED' | 'URL_INVALID' | 'URL_INSECURE' | 'ANON_KEY_REQUIRED' | 'SERVICE_ROLE_KEY_BLOCKED'
}

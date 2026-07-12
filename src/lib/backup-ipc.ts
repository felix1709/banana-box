import { invoke } from '@tauri-apps/api/core'
import type { Library } from '@/types'

export interface LegacyImportPreview {
  token: string
  promptCount: number
  categoryCount: number
  hasApiKey: boolean
  credentialConflict: boolean
  warnings: string[]
}

export interface LegacyImportCommit {
  library: Library
  promptsImported: number
  categoriesImported: number
  warnings: string[]
}

export async function inspectLegacyImport(path: string): Promise<LegacyImportPreview> {
  return await invoke<LegacyImportPreview>('inspect_legacy_import', { path })
}

export async function commitLegacyImport(
  token: string,
  overwriteCredential: boolean,
): Promise<LegacyImportCommit> {
  return await invoke<LegacyImportCommit>('commit_legacy_import', {
    token,
    overwriteCredential,
  })
}

export async function discardLegacyImportPreview(token: string): Promise<void> {
  await invoke('discard_legacy_import_preview', { token })
}

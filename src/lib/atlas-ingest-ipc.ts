import { invoke } from '@tauri-apps/api/core'

export interface AtlasIngestInput {
  localPath: string
  title?: string
  dimension?: string
  tags?: string[]
}

export async function ingestAtlasImage(input: AtlasIngestInput): Promise<string> {
  return await invoke<string>('ingest_atlas_image', { input })
}

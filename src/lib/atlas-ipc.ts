import { invoke } from '@tauri-apps/api/core'
import type { AtlasEntry } from '@/types/atlas'

export async function loadAtlasEntries(): Promise<AtlasEntry[]> {
  return await invoke<AtlasEntry[]>('load_atlas_entries', {})
}

export async function loadAtlasEntry(id: string): Promise<string | null> {
  return await invoke<string | null>('load_atlas_entry', { id })
}

export async function openAtlasFolder(): Promise<void> {
  await invoke('open_atlas_folder')
}

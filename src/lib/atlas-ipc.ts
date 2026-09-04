import { invoke } from '@tauri-apps/api/core'
import type { AtlasCategory, AtlasEntry } from '@/types/atlas'

export async function loadAtlasEntries(): Promise<AtlasEntry[]> {
  return await invoke<AtlasEntry[]>('load_atlas_entries', {})
}

export async function loadAtlasCategories(): Promise<AtlasCategory[]> {
  return await invoke<AtlasCategory[]>('load_atlas_categories', {})
}

export async function createAtlasCategory(name: string): Promise<AtlasCategory[]> {
  return await invoke<AtlasCategory[]>('create_atlas_category', { name })
}

export async function renameAtlasCategory(id: string, name: string): Promise<AtlasCategory[]> {
  return await invoke<AtlasCategory[]>('rename_atlas_category', { id, name })
}

export async function deleteAtlasCategory(id: string): Promise<AtlasCategory[]> {
  return await invoke<AtlasCategory[]>('delete_atlas_category', { id })
}

export async function setAtlasEntryCategories(
  entryId: string,
  categoryIds: string[],
): Promise<AtlasCategory[]> {
  return await invoke<AtlasCategory[]>('set_atlas_entry_categories', { entryId, categoryIds })
}

export async function deleteAtlasEntry(entryId: string): Promise<void> {
  await invoke('delete_atlas_entry', { id: entryId })
}

export async function loadAtlasEntry(id: string): Promise<string | null> {
  return await invoke<string | null>('load_atlas_entry', { id })
}

export async function openAtlasFolder(): Promise<void> {
  await invoke('open_atlas_folder')
}

export async function loadAtlasImage(id: string): Promise<string> {
  const bytes = await invoke<number[]>('load_atlas_image', { id })
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
}

export async function loadAtlasThumbnail(id: string): Promise<string> {
  const bytes = await invoke<number[]>('load_atlas_thumbnail', { id })
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
}

export async function analyzeAtlasEntry(entryId: string): Promise<string> {
  return await invoke<string>('analyze_atlas_entry', { entryId })
}

export async function getAtlasServiceStatus(): Promise<boolean> {
  return await invoke<boolean>('get_atlas_service_status')
}

export async function startAtlasService(): Promise<void> {
  await invoke('start_atlas_service', {})
}

export async function stopAtlasService(): Promise<void> {
  await invoke('stop_atlas_service', {})
}

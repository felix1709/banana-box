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

export async function loadAtlasImage(id: string): Promise<string> {
  const bytes = await invoke<number[]>('load_atlas_image', { id })
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
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

import { invoke } from '@tauri-apps/api/core'
import type { TemplateLibrary } from '@/types/template-library'

export async function loadTemplateLibrary(): Promise<TemplateLibrary> {
  return await invoke<TemplateLibrary>('load_template_library')
}

export async function loadTemplateImageBytes(imagePath: string): Promise<number[]> {
  return await invoke<number[]>('load_template_image', { imagePath })
}

export async function loadTemplateImageUrl(imagePath: string): Promise<string> {
  const bytes = await loadTemplateImageBytes(imagePath)
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
}

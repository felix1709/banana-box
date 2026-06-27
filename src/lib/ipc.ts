// src/lib/ipc.ts
// 所有 Tauri 后端调用集中在此，组件/store 只调这里的函数。
// 命令在 src-tauri/src/commands.rs 实现（Task 6/7）。

import { invoke } from '@tauri-apps/api/core'
import type { Library } from '@/types'

export async function loadLibrary(): Promise<Library> {
  return await invoke<Library>('load_library')
}

export async function saveLibrary(library: Library): Promise<void> {
  await invoke('save_library', { library })
}

export async function copyToClipboard(text: string): Promise<void> {
  await invoke('copy_to_clipboard', { text })
}

export async function saveImage(bytes: number[], ext: string): Promise<string> {
  return await invoke<string>('save_image', { bytes, ext })
}

export async function deleteImage(path: string): Promise<void> {
  await invoke('delete_image', { path })
}

// 注意：export/import 的最终实现见 Task 7（用 dialog 插件选路径后再调命令）
export async function exportLibrary(): Promise<void> {
  await invoke('export_library')
}

export async function importLibrary(): Promise<Library | null> {
  return await invoke<Library | null>('import_library')
}

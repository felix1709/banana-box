// src/lib/ipc.ts
// 所有 Tauri 后端调用集中在此，组件/store 只调这里的函数。
// 命令在 src-tauri/src/commands.rs 实现。

import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import type { Library } from '@/types'

export interface ImportFile {
  filename: string
  content: string
}

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

// 读图片字节并转成 blob URL 供 <img> 显示
export async function readImageBytes(path: string): Promise<string> {
  const bytes = await invoke<number[]>('read_image_bytes', { path })
  const blob = new Blob([new Uint8Array(bytes)])
  return URL.createObjectURL(blob)
}

export async function exportLibrary(): Promise<void> {
  const dest = await save({
    defaultPath: `banana-box-export-${new Date().toISOString().slice(0, 10).replace(/-/g, '')}.zip`,
    filters: [{ name: 'zip', extensions: ['zip'] }],
  })
  if (!dest) return
  await invoke('export_library', { dest })
}

export async function importLibrary(): Promise<Library | null> {
  const picked = await open({
    filters: [{ name: 'zip', extensions: ['zip'] }],
    multiple: false,
  })
  if (!picked || Array.isArray(picked)) return null
  return await invoke<Library>('import_library', { zipPath: picked })
}

// 批量导入：读目录下所有 .md/.txt
export async function readImportDir(dir: string): Promise<ImportFile[]> {
  return await invoke<ImportFile[]>('read_import_dir', { dir })
}

// 下载远程图片到 images/，返回相对路径
export async function downloadImage(url: string): Promise<string> {
  return await invoke<string>('download_image', { url })
}

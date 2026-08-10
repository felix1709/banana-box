// src/lib/ipc.ts
// 所有 Tauri 后端调用集中在此，组件/store 只调这里的函数。
// 命令在 src-tauri/src/commands.rs 实现。

import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import type { CloudConfig, CloudRuntimeConfig, Library, SaveCloudConfigInput } from '@/types'

export interface ImportFile {
  filename: string
  content: string
}

export interface UpdateCheckResult {
  currentVersion: string
  latestVersion: string
  updateAvailable: boolean
  releaseUrl: string
  downloadUrl: string
}

export interface ImportImageFromPathInput {
  sourcePath: string
}

export interface CompressMediaInput {
  sourcePath: string
  targetMb: number
  outputPath: string
}

export interface CompressMediaResult {
  outputPath: string
}

export interface DepthVideoInput {
  sourcePath: string
  outputPath: string
  enginePath?: string
}

export interface DepthVideoResult {
  outputPath: string
}

export interface DepthVideoEngineSetupResult {
  enginePath: string
  engineDir: string
  message: string
}

export interface DepthVideoPythonSetupResult {
  pythonVersion: string
  message: string
}

export interface SuggestCompressedOutputPathInput {
  sourcePath: string
}

export interface SuggestDepthVideoOutputPathInput {
  sourcePath: string
}

export async function loadLibrary(): Promise<Library> {
  return await invoke<Library>('load_library')
}

export async function loadCloudConfig(): Promise<CloudConfig> {
  return await invoke<CloudConfig>('load_cloud_config', {})
}

export async function loadCloudRuntimeConfig(): Promise<CloudRuntimeConfig> {
  return await invoke<CloudRuntimeConfig>('load_cloud_runtime_config', {})
}

export async function saveCloudConfig(input: SaveCloudConfigInput): Promise<CloudConfig> {
  return await invoke<CloudConfig>('save_cloud_config', { input })
}

export async function saveLibrary(library: Library): Promise<void> {
  await invoke('save_library', { library })
}

export async function copyToClipboard(text: string): Promise<void> {
  await invoke('copy_to_clipboard', { text })
}

export async function togglePanel(): Promise<void> {
  await invoke('toggle_panel')
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

// 批量导入：读目录下所有 .md/.txt
export async function readImportDir(dir: string): Promise<ImportFile[]> {
  return await invoke<ImportFile[]>('read_import_dir', { dir })
}

// 下载远程图片到 images/，返回相对路径
export async function downloadImage(url: string): Promise<string> {
  return await invoke<string>('download_image', { url })
}

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  return await invoke<UpdateCheckResult>('check_for_update')
}

export async function importImageFromPath(input: ImportImageFromPathInput): Promise<string> {
  return await invoke<string>('import_image_from_path', { input })
}

export async function compressMedia(input: CompressMediaInput): Promise<CompressMediaResult> {
  return await invoke<CompressMediaResult>('compress_media', { input })
}

export async function suggestCompressedOutputPath(
  input: SuggestCompressedOutputPathInput,
): Promise<string> {
  return await invoke<string>('suggest_compressed_output_path', { input })
}

export async function convertVideoToDepthVideo(input: DepthVideoInput): Promise<DepthVideoResult> {
  return await invoke<DepthVideoResult>('convert_video_to_depth_video', { input })
}

export async function prepareDepthVideoEngine(): Promise<DepthVideoEngineSetupResult> {
  return await invoke<DepthVideoEngineSetupResult>('prepare_depth_video_engine')
}

export async function prepareDepthVideoPython(): Promise<DepthVideoPythonSetupResult> {
  return await invoke<DepthVideoPythonSetupResult>('prepare_depth_video_python')
}

export async function suggestDepthVideoOutputPath(
  input: SuggestDepthVideoOutputPathInput,
): Promise<string> {
  return await invoke<string>('suggest_depth_video_output_path', { input })
}

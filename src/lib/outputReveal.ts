import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener'

export function outputFolderPath(outputPath: string): string {
  const trimmedPath = outputPath.trim()
  const lastSlash = Math.max(trimmedPath.lastIndexOf('\\'), trimmedPath.lastIndexOf('/'))
  if (lastSlash < 0) return trimmedPath
  if (/^[A-Za-z]:[\\/]$/.test(trimmedPath.slice(0, lastSlash + 1))) {
    return trimmedPath.slice(0, lastSlash + 1)
  }
  return trimmedPath.slice(0, lastSlash)
}

export async function revealOutputPath(outputPath: string): Promise<boolean> {
  try {
    await revealItemInDir(outputPath)
    return true
  } catch {
    try {
      await openPath(outputFolderPath(outputPath))
      return true
    } catch {
      return false
    }
  }
}

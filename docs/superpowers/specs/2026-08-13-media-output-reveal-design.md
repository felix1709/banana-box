# Media Output Reveal Design

## Goal

After Fast Compression or Depth Video finishes, help users immediately find the generated file.

## Behavior

- On successful compression, Banana Box reveals the generated output file in the system file manager.
- On successful depth-video conversion, Banana Box reveals the generated output file in the system file manager.
- If file selection reveal is unsupported or fails, Banana Box opens the output file's parent folder.
- If both automatic actions fail, the media operation remains successful and the page shows a small non-blocking hint: `文件已生成，但无法自动打开文件夹。`
- The output status area also provides a manual `打开所在文件夹` button so users can reopen the folder later.

## Implementation

- Add `src/lib/outputReveal.ts` with:
  - `outputFolderPath(outputPath: string): string`
  - `revealOutputPath(outputPath: string): Promise<boolean>`
- Use `@tauri-apps/plugin-opener`:
  - Prefer `revealItemInDir(outputPath)` to select the generated file.
  - Fall back to `openPath(parentFolder)`.
- Call `revealOutputPath(result.outputPath)` after successful compression/conversion.

## Testing

- Unit-test parent folder extraction for Windows and POSIX-style paths.
- Unit-test reveal fallback from `revealItemInDir` to `openPath`.
- Component-test Fast Compression auto reveal and manual folder button.
- Component-test Depth Video auto reveal and manual folder button.

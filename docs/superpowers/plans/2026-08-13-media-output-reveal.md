# Media Output Reveal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically reveal generated media files after compression or depth-video conversion, with a manual reopen button.

**Architecture:** Add one small frontend helper around Tauri opener APIs, then call it from the two media panels after successful output generation. Keep reveal failures non-blocking.

**Tech Stack:** Vue 3, Vitest, `@tauri-apps/plugin-opener`.

## Global Constraints

- Preserve existing compression and conversion success behavior.
- Reveal/open-folder failure must not turn a successful media operation into a failed operation.
- Do not touch unrelated historical dirty files.
- Use tests before production implementation.

---

### Task 1: Output Reveal Helper

**Files:**
- Create: `src/lib/outputReveal.ts`
- Create: `tests/lib/outputReveal.test.ts`

**Interfaces:**
- Produces: `outputFolderPath(outputPath: string): string`
- Produces: `revealOutputPath(outputPath: string): Promise<boolean>`

- [ ] Write tests for Windows and POSIX folder extraction.
- [ ] Write a test that `revealOutputPath` calls `revealItemInDir`.
- [ ] Write a test that it falls back to `openPath(parentFolder)` when reveal fails.
- [ ] Run `pnpm vitest run tests/lib/outputReveal.test.ts` and verify failure.
- [ ] Implement the helper.
- [ ] Re-run the helper tests and verify pass.

### Task 2: Fast Compression Integration

**Files:**
- Modify: `src/components/FastCompressionPanel.vue`
- Modify: `tests/components/FastCompressionPanel.test.ts`

**Interfaces:**
- Consumes: `revealOutputPath(result.outputPath): Promise<boolean>`

- [ ] Add a component test that compression success calls `revealOutputPath`.
- [ ] Add a component test that the manual `打开所在文件夹` button calls `revealOutputPath`.
- [ ] Run the Fast Compression component test and verify failure.
- [ ] Import and call the helper after successful compression.
- [ ] Add the manual button near `已输出`.
- [ ] Re-run the component test and verify pass.

### Task 3: Depth Video Integration

**Files:**
- Modify: `src/components/DepthVideoPanel.vue`
- Modify: `tests/components/DepthVideoPanel.test.ts`

**Interfaces:**
- Consumes: `revealOutputPath(result.outputPath): Promise<boolean>`

- [ ] Add a component test that depth conversion success calls `revealOutputPath`.
- [ ] Add a component test that the manual `打开所在文件夹` button calls `revealOutputPath`.
- [ ] Run the Depth Video component test and verify failure.
- [ ] Import and call the helper after successful conversion.
- [ ] Add the manual button near `已输出`.
- [ ] Re-run the component test and verify pass.

### Task 4: Verification

- [ ] Run targeted helper and component tests.
- [ ] Run `pnpm check`.
- [ ] Report evidence and note that this is not yet released unless the user asks for `提交发布`.

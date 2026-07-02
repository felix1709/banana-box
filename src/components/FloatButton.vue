<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emitTo } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

const win = getCurrentWindow()
let startX = 0
let startY = 0
let dragging = false
let unlistenDragDrop: (() => void) | null = null

type DroppedFileType = 'image' | 'video'

const imageExts = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif'])
const videoExts = new Set(['mp4', 'mov', 'webm', 'avi', 'mkv'])

function classifyFile(file: File): DroppedFileType | null {
  if (file.type.startsWith('image/')) return 'image'
  if (file.type.startsWith('video/')) return 'video'
  const ext = file.name.split('.').pop()?.toLowerCase() ?? ''
  if (imageExts.has(ext)) return 'image'
  if (videoExts.has(ext)) return 'video'
  return null
}

function classifyPath(path: string): DroppedFileType | null {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  if (imageExts.has(ext)) return 'image'
  if (videoExts.has(ext)) return 'video'
  return null
}

function fileNameFromPath(path: string) {
  return path.split(/[\\/]/).pop() || path
}

async function emitDroppedFile(filePath: string, fileName: string, fileType: DroppedFileType) {
  await invoke('show_panel')
  await emitTo('main', 'floating-file-dropped', {
    filePath,
    fileName,
    fileType,
  })
}

function onMousedown(e: MouseEvent) {
  if (e.button !== 0) return
  startX = e.screenX
  startY = e.screenY
  dragging = false
}

async function onMousemove(e: MouseEvent) {
  if (e.buttons !== 1 || dragging) return
  if (Math.abs(e.screenX - startX) > 3 || Math.abs(e.screenY - startY) > 3) {
    dragging = true
    try {
      await win.startDragging()
    } catch {
      // ignore
    }
  }
}

function onClick() {
  dragging = false
}

async function onDrop(e: DragEvent) {
  e.preventDefault()
  const file = e.dataTransfer?.files?.[0]
  if (!file) return
  const fileType = classifyFile(file)
  if (!fileType) return
  const filePath = (file as File & { path?: string }).path ?? file.name
  await emitDroppedFile(filePath, file.name, fileType)
}

onMounted(async () => {
  unlistenDragDrop = await win.onDragDropEvent((event) => {
    if (event.payload.type !== 'drop') return
    const filePath = event.payload.paths[0]
    if (!filePath) return
    const fileType = classifyPath(filePath)
    if (!fileType) return
    void emitDroppedFile(filePath, fileNameFromPath(filePath), fileType)
  })
})

onUnmounted(() => {
  unlistenDragDrop?.()
  unlistenDragDrop = null
})
</script>

<template>
  <div
    class="float-btn"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
    @click="onClick"
    @dragover.prevent
    @drop.prevent="onDrop"
  >
    🍌
  </div>
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.float-btn {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  font-family: "Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji", system-ui, sans-serif;
  line-height: 1;
}

.float-btn:hover {
  filter: drop-shadow(0 2px 5px rgba(15, 23, 42, 0.28));
}

.float-btn:active {
  transform: scale(0.96);
}
</style>

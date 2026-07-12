<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import AnimatedBananaButton from '@/components/AnimatedBananaButton.vue'
import type {
  PanelStateSnapshot,
  PanelTargetChanged,
  PanelVisibilityChanged,
} from '@/types/desktop'

const win = getCurrentWindow()
const panelOpen = ref(false)
const panelGeneration = ref(0)
let acknowledgedGeneration = -1
let startX = 0
let startY = 0
let dragging = false
let unlistenDragDrop: UnlistenFn | null = null
let unlistenPanelTarget: UnlistenFn | null = null
let unlistenPanelVisibility: UnlistenFn | null = null

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

function applyPanelTarget(payload: PanelTargetChanged) {
  if (payload.generation < panelGeneration.value) return
  panelGeneration.value = payload.generation
  panelOpen.value = payload.targetVisible
  if (!payload.targetVisible) acknowledgedGeneration = -1
}

function applyPanelSnapshot(payload: PanelStateSnapshot) {
  if (payload.generation < panelGeneration.value) return
  panelGeneration.value = payload.generation
  panelOpen.value = payload.desiredVisible
}

function applyPanelVisibility(payload: PanelVisibilityChanged) {
  if (payload.generation < panelGeneration.value) return
  if (!payload.visible) {
    panelOpen.value = false
    acknowledgedGeneration = -1
  }
}

async function emitDroppedFile(filePath: string, fileName: string, fileType: DroppedFileType) {
  await invoke('show_panel', {})
  await emitTo('main', 'floating-file-dropped', {
    filePath,
    fileName,
    fileType,
  })
}

function onMousedown(event: MouseEvent) {
  if (event.button !== 0) return
  startX = event.screenX
  startY = event.screenY
  dragging = false
}

async function onMousemove(event: MouseEvent) {
  if (event.buttons !== 1 || dragging) return
  if (Math.abs(event.screenX - startX) > 3 || Math.abs(event.screenY - startY) > 3) {
    dragging = true
    try {
      await win.startDragging()
    } catch {
      // The window manager can reject a drag while the app is being hidden.
    }
  }
}

async function onClick() {
  if (dragging) {
    dragging = false
    return
  }
  await invoke('toggle_panel', {})
}

function onFrame(frame: number) {
  if (!panelOpen.value || frame < 6 || acknowledgedGeneration === panelGeneration.value) return
  acknowledgedGeneration = panelGeneration.value
  void invoke('ack_panel_reveal', {
    generation: panelGeneration.value,
    frame,
  })
}

async function onDrop(event: DragEvent) {
  event.preventDefault()
  const file = event.dataTransfer?.files?.[0]
  if (!file) return
  const fileType = classifyFile(file)
  if (!fileType) return
  const filePath = (file as File & { path?: string }).path ?? file.name
  await emitDroppedFile(filePath, file.name, fileType)
}

onMounted(async () => {
  ;[unlistenPanelTarget, unlistenPanelVisibility, unlistenDragDrop] = await Promise.all([
    listen<PanelTargetChanged>('panel-target-changed', (event) => applyPanelTarget(event.payload)),
    listen<PanelVisibilityChanged>('panel-visibility-changed', (event) =>
      applyPanelVisibility(event.payload),
    ),
    win.onDragDropEvent((event) => {
      if (event.payload.type !== 'drop') return
      const filePath = event.payload.paths[0]
      if (!filePath) return
      const fileType = classifyPath(filePath)
      if (!fileType) return
      void emitDroppedFile(filePath, fileNameFromPath(filePath), fileType)
    }),
  ])

  try {
    applyPanelSnapshot(await invoke<PanelStateSnapshot>('get_panel_state', {}))
  } catch {
    // The normal startup sequence retries state sync after the next panel event.
  }
})

onUnmounted(() => {
  unlistenDragDrop?.()
  unlistenPanelTarget?.()
  unlistenPanelVisibility?.()
  unlistenDragDrop = null
  unlistenPanelTarget = null
  unlistenPanelVisibility = null
})
</script>

<template>
  <AnimatedBananaButton
    class="float-btn"
    :open="panelOpen"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
    @click="onClick"
    @dragover.prevent
    @drop.prevent="onDrop"
    @frame="onFrame"
  />
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent !important;
}

.float-btn {
  display: grid;
  width: 64px;
  height: 64px;
  place-items: center;
  user-select: none;
  transition:
    filter 140ms ease,
    transform 140ms ease;
}

.float-btn:hover {
  filter: drop-shadow(0 4px 10px rgba(244, 196, 48, 0.28));
}

.float-btn:active {
  transform: scale(0.96);
}
</style>

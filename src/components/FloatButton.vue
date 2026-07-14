<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import AnimatedBananaButton from '@/components/AnimatedBananaButton.vue'
import type {
  PanelStateSnapshot,
  PanelTargetChanged,
  PanelVisibilityChanged,
} from '@/types/desktop'

const win = getCurrentWindow()
const panelOpen = ref(false)
const panelGeneration = ref(0)
const activeReminder = ref<DailyTaskReminderPayload | null>(null)
const reminderExpanded = ref(false)
let acknowledgedGeneration = -1
let compactPosition: { x: number, y: number } | null = null
let startX = 0
let startY = 0
let dragging = false
let unlistenDragDrop: UnlistenFn | null = null
let unlistenPanelTarget: UnlistenFn | null = null
let unlistenPanelVisibility: UnlistenFn | null = null
let unlistenDailyReminder: UnlistenFn | null = null

type DroppedFileType = 'image' | 'video'

interface DailyTaskReminderPayload {
  taskId: string
  title: string
  body: string
  time: string
  localDate: string
}

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

function isDailyTaskReminderPayload(value: unknown): value is DailyTaskReminderPayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<DailyTaskReminderPayload>
  return (
    typeof payload.taskId === 'string' &&
    typeof payload.title === 'string' &&
    typeof payload.body === 'string' &&
    typeof payload.time === 'string' &&
    typeof payload.localDate === 'string'
  )
}

async function expandForReminder() {
  const position = await win.outerPosition()
  compactPosition = { x: position.x, y: position.y }
  await win.setSize(new LogicalSize(360, 180))
  await win.setPosition(new PhysicalPosition(position.x - 296, position.y))
  reminderExpanded.value = true
}

async function restoreReminderSize() {
  if (compactPosition) {
    await win.setPosition(new PhysicalPosition(compactPosition.x, compactPosition.y))
  }
  await win.setSize(new LogicalSize(64, 64))
  reminderExpanded.value = false
  compactPosition = null
}

async function showReminder(payload: DailyTaskReminderPayload) {
  activeReminder.value = payload
  try {
    await expandForReminder()
  } catch {
    // The reminder content still renders in browser tests or if native resizing is unavailable.
  }
}

async function dismissReminder() {
  activeReminder.value = null
  try {
    await restoreReminderSize()
  } catch {
    // Keep the UI responsive even if the native window rejects a resize.
  }
}

async function snoozeReminder(minutes: 10 | 60) {
  if (!activeReminder.value) return
  await emitTo('main', 'daily-task-reminder-snooze', {
    taskId: activeReminder.value.taskId,
    localDate: activeReminder.value.localDate,
    minutes,
  })
  await dismissReminder()
}

async function openReminderTask() {
  await invoke('show_panel', {})
  await emitTo('main', 'daily-task-reminder-open-task', activeReminder.value)
  await dismissReminder()
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
  ;[unlistenPanelTarget, unlistenPanelVisibility, unlistenDailyReminder, unlistenDragDrop] = await Promise.all([
    listen<PanelTargetChanged>('panel-target-changed', (event) => applyPanelTarget(event.payload)),
    listen<PanelVisibilityChanged>('panel-visibility-changed', (event) =>
      applyPanelVisibility(event.payload),
    ),
    listen('daily-task-reminder', (event) => {
      if (!isDailyTaskReminderPayload(event.payload)) return
      void showReminder(event.payload)
    }),
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
  unlistenDailyReminder?.()
  unlistenDragDrop = null
  unlistenPanelTarget = null
  unlistenPanelVisibility = null
  unlistenDailyReminder = null
})
</script>

<template>
  <div
    class="floating-shell"
    :class="{ 'is-reminder-expanded': reminderExpanded }"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
    @click="onClick"
    @dragover.prevent
    @drop.prevent="onDrop"
  >
    <AnimatedBananaButton
      class="float-btn"
      :open="panelOpen"
      @frame="onFrame"
    />
    <section
      v-if="activeReminder"
      class="floating-reminder"
      data-floating-reminder
      role="alertdialog"
      aria-live="assertive"
      aria-labelledby="floating-reminder-title"
      @mousedown.stop
      @click.stop
    >
      <header>
        <p>{{ activeReminder.localDate }} {{ activeReminder.time }}</p>
        <h2 id="floating-reminder-title">
          {{ activeReminder.title }}
        </h2>
      </header>
      <p>{{ activeReminder.body }}</p>
      <footer>
        <button
          type="button"
          data-action="snooze-reminder-10"
          @click="snoozeReminder(10)"
        >
          10分钟后
        </button>
        <button
          type="button"
          data-action="snooze-reminder-60"
          @click="snoozeReminder(60)"
        >
          1小时后
        </button>
        <button
          type="button"
          data-action="dismiss-reminder"
          @click="dismissReminder"
        >
          知道了
        </button>
        <button
          class="primary"
          type="button"
          @click="openReminderTask"
        >
          查看任务
        </button>
      </footer>
    </section>
  </div>
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

.floating-shell {
  position: relative;
  width: 360px;
  height: 180px;
  background: transparent;
}

.floating-shell.is-reminder-expanded .float-btn {
  position: absolute;
  top: 0;
  right: 0;
}

.floating-reminder {
  position: absolute;
  top: 8px;
  left: 8px;
  width: 276px;
  max-height: 164px;
  display: grid;
  gap: 8px;
  overflow: auto;
  padding: 11px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.98);
  box-shadow: none;
}

.floating-reminder header,
.floating-reminder p,
.floating-reminder h2 {
  margin: 0;
}

.floating-reminder header p {
  color: var(--bb-primary);
  font: 10px var(--bb-mono);
}

.floating-reminder h2 {
  margin-top: 3px;
  font-size: 14px;
  line-height: 1.3;
  overflow-wrap: anywhere;
}

.floating-reminder > p {
  color: var(--bb-text-soft);
  font-size: 12px;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.floating-reminder footer {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 7px;
}

.floating-reminder button {
  min-height: 28px;
  padding: 4px 8px;
  white-space: nowrap;
}

.floating-reminder .primary {
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary);
  color: #06231f;
  font-weight: 750;
}
</style>

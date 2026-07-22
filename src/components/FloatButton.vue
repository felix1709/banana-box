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
const activeDailyReview = ref<FloatingDailyTaskReviewPayload | null>(null)
const reviewComplete = ref<FloatingDailyTaskReviewCompletePayload | null>(null)
const reviewError = ref('')
const reminderExpanded = ref(false)
let acknowledgedGeneration = -1
let compactPosition: { x: number, y: number } | null = null
let dailyReviewAnchorSessionId: string | null = null
let startX = 0
let startY = 0
let dragging = false
let unlistenDragDrop: UnlistenFn | null = null
let unlistenPanelTarget: UnlistenFn | null = null
let unlistenPanelVisibility: UnlistenFn | null = null
let unlistenDailyReminder: UnlistenFn | null = null
let unlistenDailyReviewStart: UnlistenFn | null = null
let unlistenDailyReviewUpdate: UnlistenFn | null = null
let unlistenDailyReviewComplete: UnlistenFn | null = null
let unlistenDailyReviewError: UnlistenFn | null = null
let unlistenDailyReviewClose: UnlistenFn | null = null

type DroppedFileType = 'image' | 'video'

interface DailyTaskReminderPayload {
  taskId: string
  title: string
  body: string
  time: string
  localDate: string
}

interface FloatingDailyTaskReviewItem {
  taskId: string
  code: string
  title: string
  progress: number
  note: string
}

interface FloatingDailyTaskReviewPayload {
  sessionId: string
  localDate: string
  index: number
  total: number
  task: FloatingDailyTaskReviewItem
}

interface FloatingDailyTaskReviewCompletePayload {
  sessionId: string
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

function isFloatingDailyTaskReviewPayload(value: unknown): value is FloatingDailyTaskReviewPayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<FloatingDailyTaskReviewPayload>
  const task = payload.task as Partial<FloatingDailyTaskReviewItem> | undefined
  return (
    typeof payload.sessionId === 'string' &&
    typeof payload.localDate === 'string' &&
    typeof payload.index === 'number' &&
    typeof payload.total === 'number' &&
    !!task &&
    typeof task.taskId === 'string' &&
    typeof task.code === 'string' &&
    typeof task.title === 'string' &&
    typeof task.progress === 'number' &&
    typeof task.note === 'string'
  )
}

function isFloatingDailyTaskReviewCompletePayload(
  value: unknown,
): value is FloatingDailyTaskReviewCompletePayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<FloatingDailyTaskReviewCompletePayload>
  return typeof payload.sessionId === 'string' && typeof payload.localDate === 'string'
}

async function expandForReminder() {
  const position = await win.outerPosition()
  compactPosition = { x: position.x, y: position.y }
  dailyReviewAnchorSessionId = null
  await win.setSize(new LogicalSize(360, 180))
  await win.setPosition(new PhysicalPosition(position.x - 296, position.y))
  reminderExpanded.value = true
}

async function expandForDailyReview(sessionId: string) {
  if (!compactPosition || dailyReviewAnchorSessionId !== sessionId) {
    const position = await win.outerPosition()
    compactPosition = { x: position.x, y: position.y }
    dailyReviewAnchorSessionId = sessionId
  }
  await win.setSize(new LogicalSize(380, 220))
  await win.setPosition(new PhysicalPosition(compactPosition.x - 316, compactPosition.y))
  reminderExpanded.value = true
}

async function restoreReminderSize() {
  if (compactPosition) {
    await win.setPosition(new PhysicalPosition(compactPosition.x, compactPosition.y))
  }
  await win.setSize(new LogicalSize(64, 64))
  reminderExpanded.value = false
  compactPosition = null
  dailyReviewAnchorSessionId = null
}

async function showReminder(payload: DailyTaskReminderPayload) {
  activeReminder.value = payload
  activeDailyReview.value = null
  reviewComplete.value = null
  reviewError.value = ''
  try {
    await expandForReminder()
  } catch {
    // The reminder content still renders in browser tests or if native resizing is unavailable.
  }
}

async function showDailyReview(payload: FloatingDailyTaskReviewPayload) {
  activeReminder.value = null
  activeDailyReview.value = payload
  reviewComplete.value = null
  reviewError.value = ''
  try {
    await expandForDailyReview(payload.sessionId)
  } catch {
    // The review content still renders in browser tests or if native resizing is unavailable.
  }
}

async function showDailyReviewComplete(payload: FloatingDailyTaskReviewCompletePayload) {
  activeReminder.value = null
  activeDailyReview.value = null
  reviewComplete.value = payload
  reviewError.value = ''
  try {
    await expandForDailyReview(payload.sessionId)
  } catch {
    // The review content still renders in browser tests or if native resizing is unavailable.
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

async function closeDailyReview() {
  const sessionId = activeDailyReview.value?.sessionId ?? reviewComplete.value?.sessionId
  activeDailyReview.value = null
  reviewComplete.value = null
  reviewError.value = ''
  if (sessionId) {
    await emitTo('main', 'daily-task-review-dismiss', { sessionId })
  }
  try {
    await restoreReminderSize()
  } catch {
    // Keep the UI responsive even if the native window rejects a resize.
  }
}

async function clearDailyReviewFromMain(sessionId: string) {
  if (
    activeDailyReview.value?.sessionId !== sessionId &&
    reviewComplete.value?.sessionId !== sessionId
  ) {
    return
  }
  activeDailyReview.value = null
  reviewComplete.value = null
  reviewError.value = ''
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

async function completeReviewTask() {
  if (!activeDailyReview.value) return
  await emitTo('main', 'daily-task-review-complete-task', {
    sessionId: activeDailyReview.value.sessionId,
    taskId: activeDailyReview.value.task.taskId,
  })
}

async function delayReviewTask() {
  if (!activeDailyReview.value) return
  await emitTo('main', 'daily-task-review-delay-task', {
    sessionId: activeDailyReview.value.sessionId,
    taskId: activeDailyReview.value.task.taskId,
  })
}

async function copyDailyReviewReport() {
  if (!reviewComplete.value) return
  await emitTo('main', 'daily-task-review-copy-report', {
    sessionId: reviewComplete.value.sessionId,
  })
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
  ;[
    unlistenPanelTarget,
    unlistenPanelVisibility,
    unlistenDailyReminder,
    unlistenDailyReviewStart,
    unlistenDailyReviewUpdate,
    unlistenDailyReviewComplete,
    unlistenDailyReviewError,
    unlistenDailyReviewClose,
    unlistenDragDrop,
  ] = await Promise.all([
    listen<PanelTargetChanged>('panel-target-changed', (event) => applyPanelTarget(event.payload)),
    listen<PanelVisibilityChanged>('panel-visibility-changed', (event) =>
      applyPanelVisibility(event.payload),
    ),
    listen('daily-task-reminder', (event) => {
      if (!isDailyTaskReminderPayload(event.payload)) return
      void showReminder(event.payload)
    }),
    listen('daily-task-review-start', (event) => {
      if (!isFloatingDailyTaskReviewPayload(event.payload)) return
      void showDailyReview(event.payload)
    }),
    listen('daily-task-review-update', (event) => {
      if (!isFloatingDailyTaskReviewPayload(event.payload)) return
      void showDailyReview(event.payload)
    }),
    listen('daily-task-review-complete', (event) => {
      if (!isFloatingDailyTaskReviewCompletePayload(event.payload)) return
      void showDailyReviewComplete(event.payload)
    }),
    listen('daily-task-review-error', (event) => {
      const payload = event.payload as { message?: unknown }
      reviewError.value = typeof payload.message === 'string' ? payload.message : '操作失败，请重试'
    }),
    listen('daily-task-review-close', (event) => {
      const payload = event.payload as { sessionId?: unknown }
      if (typeof payload.sessionId !== 'string') return
      void clearDailyReviewFromMain(payload.sessionId)
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
  unlistenDailyReviewStart?.()
  unlistenDailyReviewUpdate?.()
  unlistenDailyReviewComplete?.()
  unlistenDailyReviewError?.()
  unlistenDailyReviewClose?.()
  unlistenDragDrop = null
  unlistenPanelTarget = null
  unlistenPanelVisibility = null
  unlistenDailyReminder = null
  unlistenDailyReviewStart = null
  unlistenDailyReviewUpdate = null
  unlistenDailyReviewComplete = null
  unlistenDailyReviewError = null
  unlistenDailyReviewClose = null
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
    <section
      v-if="activeDailyReview || reviewComplete"
      class="floating-reminder floating-daily-review"
      data-floating-daily-review
      role="alertdialog"
      aria-live="assertive"
      @mousedown.stop
      @click.stop
    >
      <template v-if="activeDailyReview">
        <header>
          <p>{{ activeDailyReview.localDate }} {{ activeDailyReview.index + 1 }} / {{ activeDailyReview.total }}</p>
          <h2>每日任务确认</h2>
        </header>
        <div class="daily-review-task">
          <strong>{{ activeDailyReview.task.title }}</strong>
          <span>#{{ activeDailyReview.task.code }} {{ activeDailyReview.task.progress }}%</span>
          <p v-if="activeDailyReview.task.note.trim()">
            {{ activeDailyReview.task.note }}
          </p>
        </div>
        <p
          v-if="reviewError"
          class="floating-review-error"
          role="alert"
        >
          {{ reviewError }}
        </p>
        <footer>
          <button
            class="primary"
            type="button"
            data-action="daily-review-complete"
            @click="completeReviewTask"
          >
            完成
          </button>
          <button
            type="button"
            data-action="daily-review-delay"
            @click="delayReviewTask"
          >
            延后
          </button>
          <button
            type="button"
            data-action="daily-review-close"
            @click="closeDailyReview"
          >
            关闭
          </button>
        </footer>
      </template>
      <template v-else-if="reviewComplete">
        <header>
          <p>{{ reviewComplete.localDate }}</p>
          <h2>全部确认完成</h2>
        </header>
        <p>可以复制今天的日报。</p>
        <p
          v-if="reviewError"
          class="floating-review-error"
          role="alert"
        >
          {{ reviewError }}
        </p>
        <footer>
          <button
            class="primary"
            type="button"
            data-action="daily-review-copy-report"
            @click="copyDailyReviewReport"
          >
            复制日报
          </button>
          <button
            type="button"
            data-action="daily-review-close"
            @click="closeDailyReview"
          >
            关闭
          </button>
        </footer>
      </template>
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

.floating-shell.is-reminder-expanded {
  width: 380px;
  height: 220px;
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

.floating-daily-review {
  width: 296px;
  max-height: 204px;
}

.daily-review-task {
  display: grid;
  gap: 4px;
}

.daily-review-task strong {
  color: var(--bb-text);
  font-size: 13px;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.daily-review-task span {
  color: var(--bb-primary);
  font: 10px var(--bb-mono);
}

.daily-review-task p,
.floating-review-error {
  margin: 0;
  color: var(--bb-text-soft);
  font-size: 12px;
  line-height: 1.4;
  overflow-wrap: anywhere;
}

.floating-review-error {
  color: #ffb6c0;
}

.floating-daily-review footer {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
</style>

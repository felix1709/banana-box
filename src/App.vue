<script setup lang="ts">
import { onMounted, onUnmounted, ref, watchEffect } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { Maximize2, Minimize2, RotateCcw } from '@lucide/vue'
import { useLibraryStore } from '@/stores/library'
import { useProjectsStore } from '@/stores/projects'
import { useUiStore } from '@/stores/ui'
import { readImageBytes } from '@/lib/ipc'
import SearchBar from '@/components/SearchBar.vue'
import AppSidebar from '@/components/AppSidebar.vue'
import PromptCard from '@/components/PromptCard.vue'
import PromptEditor from '@/components/PromptEditor.vue'
import SettingsModal from '@/components/SettingsModal.vue'
import ReverseImagePanel from '@/components/ReverseImagePanel.vue'
import FastCompressionPanel from '@/components/FastCompressionPanel.vue'
import FloatingActionDialog from '@/components/FloatingActionDialog.vue'
import ProjectBoardPage from '@/components/projects/ProjectBoardPage.vue'
import ProjectEditor from '@/components/projects/ProjectEditor.vue'
import DailyTasksPage from '@/components/daily/DailyTasksPage.vue'
import StoryboardPage from '@/components/storyboard/StoryboardPage.vue'

const lib = useLibraryStore()
const projects = useProjectsStore()
const ui = useUiStore()
const previewUrl = ref('')
const expandedPromptId = ref<string | null>(null)
const sortingPromptId = ref<string | null>(null)
const windowDragActive = ref(false)
const mainWindowPinned = ref(false)
const fullscreen = ref(false)
let unlistenFloatingDrop: UnlistenFn | null = null

type ResizeDirection =
  | 'North'
  | 'South'
  | 'West'
  | 'East'
  | 'NorthWest'
  | 'NorthEast'
  | 'SouthWest'
  | 'SouthEast'

const activeResizeDirection = ref<ResizeDirection | null>(null)

const resizeHandles = [
  { direction: 'North', className: 'window-resize-handle-north' },
  { direction: 'South', className: 'window-resize-handle-south' },
  { direction: 'West', className: 'window-resize-handle-west' },
  { direction: 'East', className: 'window-resize-handle-east' },
  { direction: 'NorthWest', className: 'window-resize-handle-north-west' },
  { direction: 'NorthEast', className: 'window-resize-handle-north-east' },
  { direction: 'SouthWest', className: 'window-resize-handle-south-west' },
  { direction: 'SouthEast', className: 'window-resize-handle-south-east' },
] as const

interface FloatingFileDropPayload {
  filePath: string
  fileName: string
  fileType: 'image' | 'video'
}

function isFloatingFileDropPayload(value: unknown): value is FloatingFileDropPayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<FloatingFileDropPayload>
  return (
    typeof payload.filePath === 'string' &&
    typeof payload.fileName === 'string' &&
    (payload.fileType === 'image' || payload.fileType === 'video')
  )
}

function onDragStripMouseDown(event: MouseEvent) {
  if (event.button !== 0) return
  windowDragActive.value = true
  void invoke('begin_main_window_drag')
}

function clearWindowDragActive() {
  windowDragActive.value = false
}

async function onResizeHandleMouseDown(event: MouseEvent, direction: ResizeDirection) {
  if (event.button !== 0) return
  event.preventDefault()
  event.stopPropagation()
  activeResizeDirection.value = direction
  await invoke('begin_main_window_resize')
  await getCurrentWindow().startResizeDragging(direction)
}

function clearResizeActive() {
  activeResizeDirection.value = null
}

async function toggleMainWindowPinned() {
  const nextPinned = !mainWindowPinned.value
  mainWindowPinned.value = nextPinned
  try {
    await invoke('set_main_window_pinned', { pinned: nextPinned })
  } catch {
    mainWindowPinned.value = !nextPinned
    ui.showToast('窗口常驻设置失败')
  }
}

async function setBrowserFullscreen(enabled: boolean) {
  if (enabled) {
    await document.documentElement.requestFullscreen()
    return
  }

  if (document.fullscreenElement) {
    await document.exitFullscreen()
  }
}

async function toggleFullscreen() {
  const window = getCurrentWindow()
  const nextFullscreen = !fullscreen.value
  try {
    await window.setFullscreen(nextFullscreen)
    fullscreen.value = nextFullscreen
  } catch {
    try {
      await setBrowserFullscreen(nextFullscreen)
      fullscreen.value = nextFullscreen
    } catch {
      fullscreen.value = await window.isFullscreen().catch(() => false)
      ui.showToast('无法切换全屏')
    }
  }
}

async function restoreWindowSize() {
  const window = getCurrentWindow()
  try {
    await window.setFullscreen(false)
    fullscreen.value = false
    await window.setSize(new LogicalSize(720, 520))
    await window.center()
  } catch {
    try {
      await setBrowserFullscreen(false)
      fullscreen.value = false
    } catch {
      ui.showToast('无法恢复窗口大小')
    }
  }
}

function onSortStart(id: string) {
  sortingPromptId.value = id
  expandedPromptId.value = null
}

function onSortOver(targetId: string) {
  if (!sortingPromptId.value || sortingPromptId.value === targetId) return
  lib.movePromptBefore(sortingPromptId.value, targetId)
}

function onSortEnd() {
  sortingPromptId.value = null
}

onMounted(async () => {
  await Promise.all([lib.load(), projects.load()])
  fullscreen.value = await getCurrentWindow().isFullscreen().catch(() => false)
  ui.showPanel()
  window.addEventListener('mouseup', clearResizeActive)
  unlistenFloatingDrop = await listen('floating-file-dropped', (event) => {
    if (!isFloatingFileDropPayload(event.payload)) return
    ui.showPanel()
    ui.openFloatingActionDialog(event.payload)
  })
})

onUnmounted(() => {
  window.removeEventListener('mouseup', clearResizeActive)
  unlistenFloatingDrop?.()
  unlistenFloatingDrop = null
})

watchEffect(async () => {
  if (ui.previewImage) {
    try {
      previewUrl.value = await readImageBytes(ui.previewImage)
    } catch {
      previewUrl.value = ''
    }
  }
})
</script>

<template>
  <div
    v-show="ui.panelVisible"
    class="app"
    @pointerup="onSortEnd"
    @pointercancel="onSortEnd"
  >
    <button
      v-for="handle in resizeHandles"
      :key="handle.direction"
      class="window-resize-handle"
      :class="[
        handle.className,
        { 'window-resize-handle-active': activeResizeDirection === handle.direction },
      ]"
      type="button"
      aria-hidden="true"
      tabindex="-1"
      @mousedown="onResizeHandleMouseDown($event, handle.direction)"
      @mouseup="clearResizeActive"
    />
    <div
      class="window-drag-strip"
      title="拖动窗口"
      aria-label="拖动窗口"
      :class="{ 'window-drag-strip-active': windowDragActive }"
      @mousedown="onDragStripMouseDown"
      @mouseup="clearWindowDragActive"
      @mouseleave="clearWindowDragActive"
    >
      <span aria-hidden="true" />
      <span
        class="window-drag-marker"
        aria-hidden="true"
      />
      <button
        class="window-pin-button"
        :class="{ 'window-pin-button-active': mainWindowPinned }"
        type="button"
        :title="mainWindowPinned ? '取消窗口常驻' : '窗口常驻显示'"
        :aria-label="mainWindowPinned ? '取消窗口常驻' : '窗口常驻显示'"
        :aria-pressed="mainWindowPinned"
        @mousedown.stop
        @click.stop="toggleMainWindowPinned"
      >
        <svg
          class="window-pin-icon"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            d="M14.5 4.5 19.5 9.5M8.7 13.4l-3.2 3.2M9.8 5.5l8.7 8.7M7.1 8.2l8.7 8.7"
            fill="none"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
          />
          <path
            d="M10.4 6.1 6.9 9.6l7.5 7.5 3.5-3.5-2.1-2.1 2.4-2.4-3.3-3.3-2.4 2.4-2.1-2.1Z"
            fill="currentColor"
            opacity="0.2"
          />
        </svg>
      </button>
    </div>
    <header class="topbar">
      <SearchBar />
      <div
        class="window-drag-handle"
        title="鎷栧姩绐楀彛"
        aria-hidden="true"
      />
      <button
        class="btn"
        @click="ui.openSettings()"
      >
        设置
      </button>
      <button
        class="window-command"
        type="button"
        :title="fullscreen ? '退出全屏' : '全屏显示'"
        :aria-label="fullscreen ? '退出全屏' : '全屏显示'"
        @click="toggleFullscreen"
      >
        <Minimize2
          v-if="fullscreen"
          :size="16"
        />
        <Maximize2
          v-else
          :size="16"
        />
      </button>
      <button
        class="window-command"
        type="button"
        title="恢复默认窗口大小"
        aria-label="恢复默认窗口大小"
        @click="restoreWindowSize"
      >
        <RotateCcw :size="16" />
      </button>
      <button
        v-if="false"
        class="btn primary"
        @click="ui.openEditor(null)"
      >
        新建
      </button>
    </header>
    <div class="body">
      <aside class="sidebar">
        <AppSidebar />
      </aside>
      <main class="content">
        <section
          v-if="ui.activeTool === 'prompts'"
          class="prompt-library"
        >
          <TransitionGroup
            name="prompt-reorder"
            tag="div"
            class="prompt-list scrollable-panel animated-prompt-list"
          >
            <template
              v-for="p in lib.filteredPrompts"
              :key="p.id"
            >
              <div
                v-if="sortingPromptId === p.id"
                :key="`${p.id}-placeholder`"
                class="prompt-drop-placeholder"
                aria-hidden="true"
              />
              <PromptCard
                :prompt="p"
                :expanded="expandedPromptId === p.id"
                :sorting-prompt-id="sortingPromptId"
                @expand="expandedPromptId = $event"
                @reorder-before="lib.movePromptBefore($event, p.id)"
                @sort-start="onSortStart"
                @sort-over="onSortOver"
                @sort-end="onSortEnd"
              />
            </template>
            <p
              v-if="lib.filteredPrompts.length === 0"
              key="empty"
              class="empty"
            >
              未找到匹配的提示词
            </p>
          </TransitionGroup>
        </section>
        <ReverseImagePanel v-else-if="ui.activeTool === 'reverse-image'" />
        <FastCompressionPanel v-else-if="ui.activeTool === 'compression'" />
        <ProjectBoardPage v-else-if="ui.activeTool === 'projects'" />
        <DailyTasksPage v-else-if="ui.activeTool === 'daily-tasks'" />
        <StoryboardPage v-else-if="ui.activeTool === 'storyboard'" />
      </main>
    </div>
    <PromptEditor v-if="ui.editorOpen" />
    <ProjectEditor v-if="projects.projectEditorOpen" />
    <SettingsModal v-if="ui.settingsOpen" />
    <FloatingActionDialog />
    <div
      v-if="ui.toast"
      class="toast"
    >
      {{ ui.toast }}
    </div>
    <div
      v-if="ui.previewImage"
      class="preview-mask"
      @click="ui.preview(null)"
    >
      <img
        :src="previewUrl"
        class="preview-img"
        alt="preview"
      >
    </div>
  </div>
</template>

<style scoped>
.app {
  position: relative;
  width: 100vw;
  height: 100vh;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  font-family: var(--bb-font);
  overflow: hidden;
  background:
    radial-gradient(circle at 78% 12%, rgba(102, 247, 211, 0.16), transparent 28%),
    radial-gradient(circle at 18% 96%, rgba(82, 157, 255, 0.12), transparent 36%),
    linear-gradient(135deg, rgba(7, 17, 24, 0.98), rgba(13, 24, 35, 0.98) 46%, rgba(4, 12, 19, 0.98)),
    var(--bb-bg);
  color: var(--bb-text);
  border: 1px solid rgba(123, 255, 226, 0.2);
  border-radius: var(--bb-radius-lg);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.08),
    0 28px 72px rgba(0, 0, 0, 0.5),
    0 0 0 1px rgba(5, 14, 22, 0.84);
}
.window-resize-handle {
  position: absolute;
  z-index: 20;
  min-height: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
  opacity: 0;
  transition:
    opacity 120ms ease,
    background-color 120ms ease,
    box-shadow 120ms ease;
}
.window-resize-handle:hover:not(:disabled),
.window-resize-handle-active {
  opacity: 1;
  border: 0;
  background: rgba(102, 247, 211, 0.18);
  box-shadow: 0 0 18px rgba(102, 247, 211, 0.28);
}
.window-resize-handle-north,
.window-resize-handle-south {
  left: 12px;
  right: 12px;
  height: 7px;
}
.window-resize-handle-west,
.window-resize-handle-east {
  top: 12px;
  bottom: 12px;
  width: 7px;
}
.window-resize-handle-north {
  top: 0;
  cursor: n-resize;
}
.window-resize-handle-south {
  bottom: 0;
  cursor: s-resize;
}
.window-resize-handle-west {
  left: 0;
  cursor: w-resize;
}
.window-resize-handle-east {
  right: 0;
  cursor: e-resize;
}
.window-resize-handle-north-west,
.window-resize-handle-north-east,
.window-resize-handle-south-west,
.window-resize-handle-south-east {
  width: 14px;
  height: 14px;
}
.window-resize-handle-north-west {
  top: 0;
  left: 0;
  cursor: nw-resize;
}
.window-resize-handle-north-east {
  top: 0;
  right: 0;
  cursor: ne-resize;
}
.window-resize-handle-south-west {
  bottom: 0;
  left: 0;
  cursor: sw-resize;
}
.window-resize-handle-south-east {
  right: 0;
  bottom: 0;
  cursor: se-resize;
}
.window-drag-strip {
  height: 22px;
  flex: 0 0 22px;
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr) 32px;
  align-items: center;
  justify-items: center;
  cursor: grab;
  background: rgba(4, 12, 18, 0.78);
  border-bottom: 1px solid rgba(123, 255, 226, 0.12);
  transition:
    background-color 120ms ease,
    border-color 120ms ease;
}
.window-drag-strip:hover,
.window-drag-strip-active {
  background: rgba(102, 247, 211, 0.1);
  border-bottom-color: rgba(123, 255, 226, 0.3);
}
.window-drag-strip:active {
  cursor: grabbing;
}
.window-pin-button {
  width: 24px;
  height: 18px;
  min-height: 18px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--bb-radius-sm);
  color: var(--bb-text-soft);
  cursor: pointer;
}
.window-pin-icon {
  width: 14px;
  height: 14px;
  display: block;
}
.window-pin-button:hover,
.window-pin-button-active {
  color: var(--bb-primary-strong);
  border-color: var(--bb-border-strong);
  background: var(--bb-primary-soft);
}
.window-pin-button-active {
  box-shadow: 0 0 16px rgba(102, 247, 211, 0.14);
}
.window-drag-marker {
  width: 78px;
  height: 3px;
  border-radius: 999px;
  background: rgba(102, 247, 211, 0.66);
  box-shadow:
    0 -6px 0 rgba(102, 247, 211, 0.16),
    0 6px 0 rgba(102, 247, 211, 0.16),
    0 0 18px rgba(102, 247, 211, 0.28);
  transition:
    background-color 120ms ease,
    box-shadow 120ms ease,
    transform 120ms ease;
}
.window-drag-strip:hover .window-drag-marker,
.window-drag-strip-active .window-drag-marker {
  background: var(--bb-primary-strong);
  box-shadow:
    0 -6px 0 rgba(157, 255, 233, 0.22),
    0 6px 0 rgba(157, 255, 233, 0.22),
    0 0 24px rgba(102, 247, 211, 0.42);
  transform: scaleX(1.08);
}
.topbar {
  display: flex;
  align-items: center;
  gap: var(--bb-space-2);
  padding: 9px 10px;
  border-bottom: 1px solid rgba(123, 255, 226, 0.12);
  background: rgba(6, 14, 21, 0.74);
  backdrop-filter: blur(14px);
}
.window-drag-handle {
  display: none;
  width: 28px;
  height: 28px;
  flex: 0 0 28px;
  border-radius: 6px;
  cursor: grab;
  background:
    radial-gradient(circle, #94a3b8 1.2px, transparent 1.4px) 6px 7px / 8px 8px;
}
.window-drag-handle:hover {
  background-color: rgba(102, 247, 211, 0.1);
}
.body {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
  background: rgba(4, 12, 18, 0.36);
}
.sidebar {
  width: 168px;
  flex: 0 0 168px;
  border-right: 1px solid rgba(123, 255, 226, 0.13);
  overflow-y: auto;
  background:
    linear-gradient(180deg, rgba(8, 20, 30, 0.94) 0%, rgba(5, 13, 20, 0.9) 100%);
}
.content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0;
  min-width: 0;
  background:
    linear-gradient(180deg, rgba(10, 22, 32, 0.58), rgba(5, 13, 20, 0.72));
}
.prompt-library {
  height: 100%;
  min-height: 0;
  display: block;
}
.prompt-list {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 9px;
  min-width: 0;
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.08), transparent 34%),
    linear-gradient(180deg, rgba(9, 20, 30, 0.72), rgba(5, 13, 20, 0.86));
}
.animated-prompt-list :deep(.card) {
  transition:
    transform 160ms ease,
    border-color 120ms ease,
    background-color 120ms ease,
    box-shadow 120ms ease,
    opacity 120ms ease;
}
.animated-prompt-list :deep(.card.drag-floating) {
  transition:
    border-color 120ms ease,
    background-color 120ms ease,
    box-shadow 120ms ease,
    opacity 120ms ease;
}
.prompt-drop-placeholder {
  flex: 0 0 104px;
  height: 104px;
  min-height: 104px;
  border: 1px dashed rgba(102, 247, 211, 0.46);
  border-radius: var(--bb-radius-md);
  background: var(--bb-primary-soft);
  box-sizing: border-box;
}
.prompt-reorder-move,
.prompt-reorder-enter-active,
.prompt-reorder-leave-active {
  transition:
    transform 160ms ease,
    opacity 120ms ease;
}
.prompt-reorder-enter-from,
.prompt-reorder-leave-to {
  opacity: 0;
}
.empty {
  color: var(--bb-text-muted);
  text-align: center;
  margin-top: 32px;
  padding: 18px;
  border: 1px dashed var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
}
.btn {
  padding: 5px 10px;
  cursor: pointer;
  border-color: rgba(102, 247, 211, 0.22);
  background: rgba(102, 247, 211, 0.09);
  color: var(--bb-text);
}
.btn.primary {
  font-weight: bold;
}
.window-command {
  display: grid;
  width: 28px;
  min-height: 28px;
  place-items: center;
  padding: 0;
}
.toast {
  position: fixed;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(5, 14, 22, 0.96);
  color: var(--bb-text);
  padding: 7px 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 999px;
  box-shadow: var(--bb-shadow-floating);
  font-size: 12px;
}
.preview-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.78);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
  backdrop-filter: blur(2px);
}
.preview-img {
  max-width: 90%;
  max-height: 90%;
  width: auto;
  height: auto;
  object-fit: contain;
  border-radius: var(--bb-radius-md);
  border: 1px solid var(--bb-border-strong);
  box-shadow: var(--bb-shadow-dialog);
}
</style>

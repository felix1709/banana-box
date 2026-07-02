<script setup lang="ts">
import { onMounted, onUnmounted, ref, watchEffect } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useLibraryStore } from '@/stores/library'
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

const lib = useLibraryStore()
const ui = useUiStore()
const previewUrl = ref('')
const expandedPromptId = ref<string | null>(null)
const sortingPromptId = ref<string | null>(null)
let unlistenFloatingDrop: UnlistenFn | null = null

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

function onTopbarPointerDown(event: PointerEvent) {
  if (event.button !== 0) return
  const target = event.target
  if (!(target instanceof Element)) return
  if (target.closest('button, input, textarea, select, a, [data-no-window-drag]')) return
  void getCurrentWindow().startDragging()
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
  await lib.load()
  ui.showPanel()
  unlistenFloatingDrop = await listen('floating-file-dropped', (event) => {
    if (!isFloatingFileDropPayload(event.payload)) return
    ui.showPanel()
    ui.openFloatingActionDialog(event.payload)
  })
})

onUnmounted(() => {
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
    <header
      class="topbar"
      @pointerdown="onTopbarPointerDown"
    >
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
        <FastCompressionPanel v-else />
      </main>
    </div>
    <PromptEditor v-if="ui.editorOpen" />
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
  width: 720px;
  height: 520px;
  display: flex;
  flex-direction: column;
  font-family: system-ui, sans-serif;
  overflow: hidden;
  background: #fff;
  color: #1f2937;
  border: 1px solid #d7dde7;
  border-radius: 8px;
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.24);
}
.topbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-bottom: 1px solid #eee;
  background: #f8fafc;
}
.window-drag-handle {
  width: 28px;
  height: 28px;
  flex: 0 0 28px;
  border-radius: 6px;
  cursor: grab;
  background:
    radial-gradient(circle, #94a3b8 1.2px, transparent 1.4px) 6px 7px / 8px 8px;
}
.window-drag-handle:hover {
  background-color: #eef2f7;
}
.body {
  flex: 1;
  display: flex;
  overflow: hidden;
  min-height: 0;
}
.sidebar {
  width: 160px;
  flex: 0 0 160px;
  border-right: 1px solid #eee;
  overflow-y: auto;
  background: #f9fafb;
}
.content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0;
  min-width: 0;
  background: #fff;
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
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
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
  border: 1px dashed #cbd5e1;
  border-radius: 6px;
  background: #f8fafc;
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
  color: #999;
  text-align: center;
  margin-top: 32px;
}
.btn {
  cursor: pointer;
}
.btn.primary {
  font-weight: bold;
}
.toast {
  position: fixed;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  background: #333;
  color: #fff;
  padding: 6px 12px;
  border-radius: 6px;
}
.preview-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}
.preview-img {
  max-width: 90%;
  max-height: 90%;
  width: auto;
  height: auto;
  object-fit: contain;
}
</style>

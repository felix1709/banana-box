<script setup lang="ts">
import { computed, ref, watchEffect, onMounted, onUnmounted, type CSSProperties } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes, saveImage } from '@/lib/ipc'
import type { Prompt } from '@/types'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = withDefaults(
  defineProps<{ prompt: Prompt; expanded?: boolean | null; sortingPromptId?: string | null }>(),
  {
  expanded: null,
  sortingPromptId: null,
  },
)
const emit = defineEmits<{
  expand: [id: string | null]
  'reorder-before': [draggedId: string]
  'sort-start': [id: string]
  'sort-over': [id: string]
  'sort-end': []
}>()
const lib = useLibraryStore()
const ui = useUiStore()
const url = ref('')
const localExpanded = ref(false)
const showConfirm = ref(false)
const showCategoryPicker = ref(false)
const dragReady = ref(false)
const dragOffsetX = ref(0)
const dragOffsetY = ref(0)
const dragFrame = ref<{ left: number; top: number; width: number; height: number } | null>(null)
const suppressNextClick = ref(false)
const cardRef = ref<HTMLElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const isExpanded = computed(() => props.expanded ?? localExpanded.value)
const currentPrompt = computed(
  () => lib.library.prompts.find((prompt) => prompt.id === props.prompt.id) ?? props.prompt,
)
const dragStyle = computed<CSSProperties | undefined>(() =>
  dragReady.value && dragFrame.value
    ? {
        position: 'fixed',
        left: `${dragFrame.value.left}px`,
        top: `${dragFrame.value.top}px`,
        width: `${dragFrame.value.width}px`,
        height: `${dragFrame.value.height}px`,
        transform: `translate3d(${dragOffsetX.value}px, ${dragOffsetY.value}px, 0px)`,
      }
    : undefined,
)
let longPressTimer: ReturnType<typeof setTimeout> | null = null
let lastSortOverId: string | null = null
let dragStartX = 0
let dragStartY = 0

watchEffect(async () => {
  if (props.prompt.image) {
    try {
      url.value = await readImageBytes(props.prompt.image)
    } catch {
      url.value = ''
    }
  } else {
    url.value = ''
  }
})

function onDocClick(e: MouseEvent) {
  if (cardRef.value && !cardRef.value.contains(e.target as Node)) {
    localExpanded.value = false
    showCategoryPicker.value = false
    emit('expand', null)
  }
}

function onPaste(e: ClipboardEvent) {
  if (!isExpanded.value) return
  const file = Array.from(e.clipboardData?.files ?? []).find((item) =>
    item.type.startsWith('image/'),
  )
  if (file) void attachImage(file)
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('paste', onPaste)
})

onUnmounted(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('paste', onPaste)
  clearLongPressTimer()
  removeSortDocumentListeners()
})

function clearLongPressTimer() {
  if (!longPressTimer) return
  clearTimeout(longPressTimer)
  longPressTimer = null
}

function addSortDocumentListeners() {
  document.addEventListener('pointermove', onDocumentPointerMove)
  document.addEventListener('pointerup', onDocumentPointerUp)
  document.addEventListener('pointercancel', onDocumentPointerUp)
}

function removeSortDocumentListeners() {
  document.removeEventListener('pointermove', onDocumentPointerMove)
  document.removeEventListener('pointerup', onDocumentPointerUp)
  document.removeEventListener('pointercancel', onDocumentPointerUp)
}

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0) return
  e.preventDefault()
  clearLongPressTimer()
  dragReady.value = false
  dragOffsetX.value = 0
  dragOffsetY.value = 0
  dragFrame.value = null
  dragStartX = e.clientX
  dragStartY = e.clientY
  longPressTimer = setTimeout(() => {
    const rect = cardRef.value?.getBoundingClientRect()
    dragFrame.value = rect
      ? {
          left: rect.left,
          top: rect.top,
          width: rect.width,
          height: rect.height,
        }
      : null
    dragReady.value = true
    lastSortOverId = null
    addSortDocumentListeners()
    emit('sort-start', props.prompt.id)
  }, 400)
}

function onPointerUp() {
  clearLongPressTimer()
  finishSort()
}

function onPointerLeave() {
  if (!dragReady.value) clearLongPressTimer()
}

function onDocumentPointerUp() {
  clearLongPressTimer()
  finishSort()
}

function onDocumentPointerMove(e: PointerEvent) {
  if (!dragReady.value) return
  dragOffsetX.value = e.clientX - dragStartX
  dragOffsetY.value = e.clientY - dragStartY
  const target = document.elementFromPoint?.(e.clientX, e.clientY)
  const targetCard = target?.closest<HTMLElement>('[data-prompt-card-id]')
  const targetId = targetCard?.dataset.promptCardId
  if (!targetId || targetId === props.prompt.id || targetId === lastSortOverId) return
  lastSortOverId = targetId
  emit('sort-over', targetId)
}

function finishSort() {
  if (dragReady.value) {
    suppressNextClick.value = true
    emit('sort-end')
  }
  dragReady.value = false
  dragOffsetX.value = 0
  dragOffsetY.value = 0
  dragFrame.value = null
  lastSortOverId = null
  removeSortDocumentListeners()
}

function onClick(e: MouseEvent) {
  if (suppressNextClick.value) {
    suppressNextClick.value = false
    return
  }
  if (e.detail > 1) return
  if (e.button !== 0) return
  const nextExpanded = !isExpanded.value
  localExpanded.value = nextExpanded
  showCategoryPicker.value = false
  emit('expand', nextExpanded ? props.prompt.id : null)
}

function onDoubleClick() {
  void onCopy()
}

async function onCopy() {
  localExpanded.value = false
  showCategoryPicker.value = false
  emit('expand', null)
  await lib.copyPrompt(props.prompt.id)
  ui.showToast('已复制 ✓')
  localExpanded.value = false
  showCategoryPicker.value = false
  emit('expand', null)
}

function onEdit() {
  ui.openEditor(props.prompt.id)
  localExpanded.value = false
  showCategoryPicker.value = false
  emit('expand', null)
}

function onDelete() {
  localExpanded.value = false
  showCategoryPicker.value = false
  showConfirm.value = true
  emit('expand', null)
}

function confirmDelete() {
  lib.deletePrompt(props.prompt.id)
  showConfirm.value = false
}

function onToggleCategory() {
  showCategoryPicker.value = !showCategoryPicker.value
}

function onSelectCategory(categoryId: string | null) {
  lib.updatePrompt(props.prompt.id, { categoryId })
  showCategoryPicker.value = false
}

function onToggleFavorite() {
  lib.toggleFavorite(props.prompt.id)
}

function onPreview() {
  if (props.prompt.image) ui.preview(props.prompt.image)
}

function onThumbClick() {
  if (props.prompt.image) {
    onPreview()
  } else {
    fileInput.value?.click()
  }
}

async function attachImage(file: File) {
  if (!file.type.startsWith('image/')) return
  const ext = file.name.split('.').pop()?.toLowerCase() || 'png'
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  try {
    const path = await saveImage(bytes, ext)
    lib.updatePrompt(props.prompt.id, { image: path })
    ui.showToast('已设置参考图')
  } catch {
    ui.showToast('设图失败')
  }
}

async function onPickImage(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) await attachImage(file)
  if (fileInput.value) fileInput.value.value = ''
}

async function onDrop(e: DragEvent) {
  e.preventDefault()
  const draggedId =
    typeof e.dataTransfer?.getData === 'function' ? e.dataTransfer.getData('text/plain') : ''
  if (draggedId) {
    emit('reorder-before', draggedId)
    dragReady.value = false
    return
  }
  const file = e.dataTransfer?.files?.[0]
  if (file) await attachImage(file)
}

function onDragStart(e: DragEvent) {
  e.preventDefault()
}

function onDragEnd() {
  clearLongPressTimer()
  finishSort()
}

function onPointerEnter() {
  if (!props.sortingPromptId || props.sortingPromptId === props.prompt.id) return
  emit('sort-over', props.prompt.id)
}
</script>

<template>
  <div
    ref="cardRef"
    class="card"
    :data-prompt-card-id="prompt.id"
    :style="dragStyle"
    :class="[
      isExpanded ? 'expanded' : 'collapsed',
      {
        'category-menu-can-overflow': isExpanded,
        'expanded-auto-height-card': isExpanded,
        'flow-expanded-card': isExpanded,
        'fixed-size-prompt-card': !isExpanded,
        'drag-ready': dragReady,
        'drag-floating': dragReady,
      },
    ]"
    tabindex="0"
    draggable="false"
    @pointerdown.stop="onPointerDown"
    @pointerup.stop="onPointerUp"
    @pointercancel.stop="onPointerUp"
    @pointerleave.stop="onPointerLeave"
    @pointerenter="onPointerEnter"
    @click.stop="onClick"
    @dblclick.stop="onDoubleClick"
    @dragstart="onDragStart"
    @dragend="onDragEnd"
    @dragover.prevent
    @drop.prevent="onDrop"
  >
    <button
      type="button"
      class="favorite-button icon-only-star"
      :class="{ active: currentPrompt.favorite }"
      :aria-pressed="currentPrompt.favorite"
      title="Favorite"
      @mousedown.stop
      @pointerdown.stop
      @click.stop.prevent="onToggleFavorite"
    >
      ★
    </button>
    <div
      class="card-main fixed-card-layout"
      :class="{ 'flow-card-main': isExpanded }"
    >
      <div
        class="text-pane"
        :class="{ 'flow-text-pane': isExpanded }"
      >
        <div class="title">
          {{ prompt.title }}
        </div>
        <div
          class="content fixed-three-line-content"
          :class="{ 'full-content': isExpanded, 'flow-content': isExpanded }"
        >
          {{ prompt.content }}
        </div>
        <div
          class="tags"
          :class="{ 'full-tags': isExpanded }"
        >
          <span
            v-for="t in prompt.tags"
            :key="t"
            class="tag"
            :class="{ 'full-tag': isExpanded }"
          >{{ t }}</span>
        </div>
        <div
          v-if="isExpanded"
          class="action-panel"
        >
          <div class="actions">
            <button
              @pointerdown.stop
              @click.stop="onDelete"
            >
              删除
            </button>
            <button
              @pointerdown.stop
              @click.stop="onEdit"
            >
              编辑
            </button>
            <button
              @pointerdown.stop
              @click.stop="onToggleCategory"
            >
              分类
            </button>
            <button
              @pointerdown.stop
              @click.stop="onCopy"
            >
              复制
            </button>
          </div>

          <div
            v-if="showCategoryPicker"
            class="category-menu"
            role="menu"
            @pointerdown.stop
            @click.stop
          >
            <button
              type="button"
              class="category-menu-item"
              :class="{ active: !prompt.categoryId }"
              role="menuitem"
              @click.stop="onSelectCategory(null)"
            >
              <span class="category-dot empty" />
              未分类
            </button>
            <button
              v-for="c in lib.categories"
              :key="c.id"
              type="button"
              class="category-menu-item"
              :class="{ active: prompt.categoryId === c.id }"
              role="menuitem"
              @click.stop="onSelectCategory(c.id)"
            >
              <span
                class="category-dot"
                :style="{ backgroundColor: c.color }"
              />
              {{ c.name }}
            </button>
          </div>
        </div>
      </div>

      <button
        type="button"
        class="thumb-zone fit-box prompt-preview-thumb stable-preview-thumb"
        :class="{ empty: !url }"
        title="点击上传，拖拽或粘贴图片；已有图片时点击预览原始比例"
        @pointerdown.stop
        @click.stop="onThumbClick"
      >
        <img
          v-if="url"
          class="thumb fit-contain"
          :src="url"
          alt="参考图"
        >
        <span v-else>上传图片</span>
      </button>
      <input
        ref="fileInput"
        class="file-input"
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        @change="onPickImage"
      >
    </div>

    <ConfirmDialog
      :visible="showConfirm"
      :message="`删除提示词「${prompt.title}」？`"
      @confirm="confirmDelete"
      @cancel="showConfirm = false"
    />
  </div>
</template>

<style scoped>
.card {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  padding: 7px;
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
  overflow: hidden;
  box-sizing: border-box;
  height: 104px;
  min-height: 104px;
  max-height: 104px;
  max-width: 100%;
  background: var(--bb-surface);
  position: relative;
  transition:
    border-color 120ms ease,
    background-color 120ms ease,
    box-shadow 120ms ease,
    transform 120ms ease;
  box-shadow: 0 1px 0 rgba(15, 23, 42, 0.03);
}
.card.collapsed {
  height: 104px;
}
.card.expanded {
  border-color: #b7c7dd;
  background: var(--bb-surface);
  padding-bottom: 7px;
  height: auto;
  min-height: 104px;
  max-height: none;
  overflow: visible;
  position: relative;
  z-index: 2;
  box-shadow: var(--bb-shadow-card);
}
.card.flow-expanded-card {
  height: auto;
  min-height: 104px;
  max-height: none;
  flex: 0 0 auto;
  align-self: stretch;
}
.card.drag-ready {
  cursor: grabbing;
}
.card.drag-floating {
  z-index: 30;
  pointer-events: none;
  user-select: none;
  opacity: 0.94;
  border-color: #8ea6c2;
  box-shadow: var(--bb-shadow-floating);
  will-change: transform;
}
.card:hover {
  border-color: #c6d4e5;
  background: #fbfdff;
  box-shadow: var(--bb-shadow-sm);
}
.card-main {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 96px;
  gap: 8px;
  align-items: start;
  min-height: 100%;
}
.card.expanded .card-main {
  grid-template-columns: minmax(0, 1fr) 96px;
  height: auto;
  min-height: 0;
}
.card-main.flow-card-main {
  height: auto;
  min-height: auto;
  grid-auto-rows: max-content;
  align-items: start;
}
.text-pane {
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.card.expanded .text-pane {
  overflow: visible;
}
.text-pane.flow-text-pane {
  height: auto;
  min-height: 0;
  overflow: visible;
}
.title {
  font-weight: 600;
  font-size: 13px;
  color: var(--bb-text);
  letter-spacing: 0;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding-right: 20px;
}
.content {
  color: var(--bb-text-muted);
  font-size: 11px;
  line-height: 1.35;
  margin: 4px 0;
  height: 4.05em;
  max-height: 4.05em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.card.expanded .content {
  height: auto;
  max-height: none;
  overflow: visible;
  display: block;
  -webkit-line-clamp: unset;
}
.content.full-content {
  height: auto;
  max-height: none;
  overflow: visible;
  display: block;
  -webkit-line-clamp: unset;
}
.content.flow-content {
  position: static;
  min-height: 0;
  height: auto;
  max-height: none;
  overflow: visible;
}
.tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  min-width: 0;
  max-height: 16px;
  overflow: hidden;
}
.tags.full-tags,
.card.expanded .tags {
  max-height: none;
  overflow: visible;
}
.tag {
  border: 1px solid #dbe5f0;
  background: var(--bb-surface-soft);
  border-radius: 999px;
  padding: 1px 6px;
  font-size: 10px;
  color: #475569;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tag.full-tag,
.card.expanded .tag {
  overflow: visible;
  text-overflow: clip;
  white-space: normal;
  overflow-wrap: anywhere;
}
.thumb-zone {
  width: 96px;
  height: 72px;
  min-height: 72px;
  max-height: 72px;
  aspect-ratio: 4 / 3;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  padding: 0;
  overflow: hidden;
  background:
    linear-gradient(135deg, rgba(248, 250, 252, 0.92), rgba(241, 245, 249, 0.92)),
    var(--bb-surface-soft);
  color: var(--bb-text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  align-self: start;
  justify-self: end;
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.55);
}
.favorite-button {
  position: absolute;
  top: 7px;
  right: 7px;
  z-index: 4;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--bb-text-soft);
  cursor: pointer;
  font-size: 0;
  line-height: 1;
  padding: 0;
}
.favorite-button:hover {
  background: transparent;
  color: var(--bb-favorite);
}
.favorite-button.active {
  background: transparent;
  color: var(--bb-favorite);
}
.favorite-button::before {
  content: "☆";
  font-size: 14px;
  line-height: 1;
}
.favorite-button.active::before {
  content: "★";
}
.card.expanded .thumb-zone {
  width: 96px;
  height: 72px;
}
.thumb-zone.empty {
  border-style: dashed;
}
.thumb-zone:hover {
  border-color: #b7c7dd;
  background: var(--bb-primary-soft);
}
.thumb {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: var(--bb-surface-soft);
}
.file-input {
  display: none;
}
.action-panel {
  position: relative;
}
.actions {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 5px;
  margin-top: 8px;
  padding-top: 4px;
}
.actions button {
  min-height: 25px;
  padding: 3px 6px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-xs);
  background: var(--bb-surface);
  cursor: pointer;
  font-size: 12px;
  color: var(--bb-text);
}
.actions button:hover {
  border-color: var(--bb-border-strong);
  background: var(--bb-surface-soft);
}
.category-menu {
  position: absolute;
  top: calc(100% + 4px);
  right: 25%;
  z-index: 20;
  width: min(168px, 75vw);
  max-height: 196px;
  overflow: auto;
  padding: 4px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-floating);
}
.category-menu-item {
  width: 100%;
  min-height: 28px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 4px 8px;
  border: 0;
  border-radius: var(--bb-radius-xs);
  background: transparent;
  color: var(--bb-text);
  cursor: pointer;
  font-size: 12px;
  line-height: 1.2;
  text-align: left;
}
.category-menu-item:hover,
.category-menu-item.active {
  background: var(--bb-primary-soft);
}
.category-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  flex: 0 0 auto;
}
.category-dot.empty {
  border: 1px solid #a8b3c2;
  background: transparent;
}
</style>

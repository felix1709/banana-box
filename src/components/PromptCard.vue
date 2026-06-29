<script setup lang="ts">
import { computed, ref, watchEffect, onMounted, onUnmounted } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes, saveImage } from '@/lib/ipc'
import type { Prompt } from '@/types'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = withDefaults(defineProps<{ prompt: Prompt; expanded?: boolean | null }>(), {
  expanded: null,
})
const emit = defineEmits<{
  expand: [id: string | null]
}>()
const lib = useLibraryStore()
const ui = useUiStore()
const url = ref('')
const localExpanded = ref(false)
const showConfirm = ref(false)
const showCategoryPicker = ref(false)
const cardRef = ref<HTMLElement | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const isExpanded = computed(() => props.expanded ?? localExpanded.value)

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
})

function onClick(e: MouseEvent) {
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
  const file = e.dataTransfer?.files?.[0]
  if (file) await attachImage(file)
}
</script>

<template>
  <div
    ref="cardRef"
    class="card"
    :class="isExpanded ? 'expanded' : 'collapsed'"
    tabindex="0"
    @pointerdown.stop
    @click.stop="onClick"
    @dblclick.stop="onDoubleClick"
    @dragover.prevent
    @drop.prevent="onDrop"
  >
    <div class="card-main">
      <div class="text-pane">
        <div class="title">
          📄 {{ prompt.title }}
        </div>
        <div class="content">
          {{ prompt.content }}
        </div>
        <div class="tags">
          <span
            v-for="t in prompt.tags"
            :key="t"
            class="tag"
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
        class="thumb-zone"
        :class="{ empty: !url }"
        title="点击上传，拖拽或粘贴图片；已有图片时点击预览原始比例"
        @pointerdown.stop
        @click.stop="onThumbClick"
      >
        <img
          v-if="url"
          class="thumb"
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
  border: 1px solid #eee;
  border-radius: 6px;
  padding: 5px;
  cursor: pointer;
  overflow: hidden;
  max-width: 100%;
  background: #fff;
  transition:
    border-color 120ms ease,
    background-color 120ms ease;
}
.card.expanded {
  border-color: #cbd5e1;
  background: #fff;
  padding-bottom: 7px;
  overflow: visible;
  position: relative;
  z-index: 2;
}
.card:hover {
  background: #fafafa;
}
.card-main {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 64px;
  gap: 6px;
  align-items: start;
}
.card.expanded .card-main {
  grid-template-columns: minmax(0, 2fr) minmax(112px, 1fr);
}
.text-pane {
  min-width: 0;
}
.title {
  font-weight: 600;
  font-size: 13px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.content {
  color: #555;
  font-size: 11px;
  line-height: 1.3;
  margin: 3px 0;
  max-height: 2.6em;
  overflow: hidden;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.card.expanded .content {
  max-height: 5.2em;
  -webkit-line-clamp: 4;
}
.tags {
  display: flex;
  gap: 3px;
  flex-wrap: wrap;
  min-width: 0;
}
.tag {
  background: #eee;
  border-radius: 3px;
  padding: 0 4px;
  font-size: 10px;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.thumb-zone {
  width: 64px;
  aspect-ratio: 1 / 1;
  border: 1px solid #ddd;
  border-radius: 6px;
  padding: 0;
  overflow: hidden;
  background: #f8fafc;
  color: #777;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  justify-self: end;
}
.card.expanded .thumb-zone {
  width: 100%;
}
.thumb-zone.empty {
  border-style: dashed;
}
.thumb-zone:hover {
  background: #f0f4f8;
}
.thumb {
  width: 100%;
  height: 100%;
  object-fit: cover;
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
  gap: 4px;
  margin-top: 6px;
  padding-top: 2px;
}
.actions button {
  min-height: 23px;
  padding: 2px 6px 3px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background: #fff;
  cursor: pointer;
  font-size: 12px;
}
.actions button:hover {
  background: #f0f0f0;
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
  border: 1px solid #d6dde8;
  border-radius: 6px;
  background: #fff;
  box-shadow: 0 8px 22px rgb(15 23 42 / 16%);
}
.category-menu-item {
  width: 100%;
  min-height: 28px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 4px 8px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #1f2937;
  cursor: pointer;
  font-size: 12px;
  line-height: 1.2;
  text-align: left;
}
.category-menu-item:hover,
.category-menu-item.active {
  background: #eef2f7;
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

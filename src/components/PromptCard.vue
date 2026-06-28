<script setup lang="ts">
import { ref, watchEffect, onMounted, onUnmounted } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes, saveImage } from '@/lib/ipc'
import type { Prompt } from '@/types'
import ConfirmDialog from '@/components/ConfirmDialog.vue'

const props = defineProps<{ prompt: Prompt }>()
const lib = useLibraryStore()
const ui = useUiStore()
const url = ref('')
const showActions = ref(false)
const showConfirm = ref(false)
const cardRef = ref<HTMLElement | null>(null)
let clickTimer: number | null = null

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

// 点卡片外部关闭操作面板
function onDocClick(e: MouseEvent) {
  if (cardRef.value && !cardRef.value.contains(e.target as Node)) {
    showActions.value = false
  }
}
onMounted(() => document.addEventListener('click', onDocClick))
onUnmounted(() => document.removeEventListener('click', onDocClick))

// 单击 toggle 操作面板；双击快捷复制（220ms 延迟区分）
function onClick() {
  if (clickTimer !== null) {
    clearTimeout(clickTimer)
    clickTimer = null
    void onCopy()
    return
  }
  clickTimer = window.setTimeout(() => {
    clickTimer = null
    showActions.value = !showActions.value
  }, 220)
}

async function onCopy() {
  await lib.copyPrompt(props.prompt.id)
  ui.showToast('已复制 ✓')
  showActions.value = false
}

function onEdit() {
  ui.openEditor(props.prompt.id)
  showActions.value = false
}

function onDelete() {
  showActions.value = false
  showConfirm.value = true
}

function confirmDelete() {
  lib.deletePrompt(props.prompt.id)
  showConfirm.value = false
}

function onPreview() {
  if (props.prompt.image) ui.preview(props.prompt.image)
}

// 拖拽图片到卡片 → 设为参考图
async function onDrop(e: DragEvent) {
  e.preventDefault()
  const file = e.dataTransfer?.files?.[0]
  if (!file || !file.type.startsWith('image/')) return
  const ext = file.name.split('.').pop()?.toLowerCase() || 'png'
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  try {
    const path = await saveImage(bytes, ext)
    lib.updatePrompt(props.prompt.id, { image: path })
    ui.showToast('已设参考图')
  } catch {
    ui.showToast('设图失败')
  }
}
</script>

<template>
  <div
    ref="cardRef"
    class="card"
    @click.stop="onClick"
    @dragover.prevent
    @drop.prevent="onDrop"
  >
    <div class="row">
      <div class="title">
        📌 {{ prompt.title }}
      </div>
      <img
        v-if="url"
        class="thumb"
        :src="url"
        alt="ref"
        @click.stop="onPreview"
      >
      <div
        v-else
        class="thumb-placeholder"
        title="拖拽图片到此处设参考图"
      >
        拖图
      </div>
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
      v-if="showActions"
      class="actions"
    >
      <button @click.stop="onCopy">
        📋 复制
      </button>
      <button @click.stop="onEdit">
        ✏️ 编辑
      </button>
      <button @click.stop="onDelete">
        🗑 删除
      </button>
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
  border-radius: 8px;
  padding: 10px;
  cursor: pointer;
}
.card:hover {
  background: #fafafa;
}
.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-weight: 600;
}
.content {
  color: #555;
  font-size: 13px;
  margin: 4px 0;
}
.tags {
  display: flex;
  gap: 4px;
}
.tag {
  background: #eee;
  border-radius: 4px;
  padding: 1px 6px;
  font-size: 11px;
}
.thumb {
  width: 48px;
  height: 48px;
  object-fit: cover;
  border-radius: 6px;
}
.thumb-placeholder {
  width: 48px;
  height: 48px;
  border: 1px dashed #ccc;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: #aaa;
}
.actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.actions button {
  padding: 4px 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background: #fff;
  cursor: pointer;
  font-size: 13px;
}
.actions button:hover {
  background: #f0f0f0;
}
</style>

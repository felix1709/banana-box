<script setup lang="ts">
import { ref, computed } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { saveImage } from '@/lib/ipc'
import type { Prompt } from '@/types'

const lib = useLibraryStore()
const ui = useUiStore()

const editing = computed(
  () => lib.library.prompts.find((p) => p.id === ui.editingPromptId) || null,
)

const form = ref({
  title: editing.value?.title ?? ui.editorPrefill?.title ?? '',
  content: editing.value?.content ?? ui.editorPrefill?.content ?? '',
  categoryId: (editing.value?.categoryId ?? ui.editorPrefill?.categoryId ?? null) as string | null,
  tags: editing.value?.tags.join(', ') ?? ui.editorPrefill?.tags?.join(', ') ?? '',
  image: (editing.value?.image ?? ui.editorPrefill?.image ?? null) as string | null,
})

async function onPickImage(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  const ext = file.name.split('.').pop()?.toLowerCase() || 'png'
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  form.value.image = await saveImage(bytes, ext)
}

function onSave() {
  const tags = form.value.tags
    .split(',')
    .map((t) => t.trim())
    .filter(Boolean)
  const payload = {
    title: form.value.title,
    content: form.value.content,
    categoryId: form.value.categoryId,
    tags,
    image: form.value.image,
  }
  if (editing.value) {
    lib.updatePrompt(editing.value.id, payload)
  } else {
    lib.addPrompt(payload as Omit<Prompt, 'id' | 'createdAt' | 'updatedAt'>)
  }
  ui.closeEditor()
}
</script>

<template>
  <div
    class="mask"
    @click.self="ui.closeEditor()"
  >
    <div class="dialog">
      <h3>{{ editing ? '编辑' : '新建' }}提示词</h3>
      <input
        v-model="form.title"
        placeholder="标题"
      >
      <textarea
        v-model="form.content"
        placeholder="提示词内容"
        rows="5"
      />
      <select v-model="form.categoryId">
        <option :value="null">
          未分类
        </option>
        <option
          v-for="c in lib.categories"
          :key="c.id"
          :value="c.id"
        >
          {{ c.name }}
        </option>
      </select>
      <input
        v-model="form.tags"
        placeholder="标签，逗号分隔"
      >
      <input
        type="file"
        accept="image/png,image/jpeg,image/webp"
        @change="onPickImage"
      >
      <div v-if="form.image">
        已附图：{{ form.image }}
      </div>
      <div class="actions">
        <button @click="ui.closeEditor()">
          取消
        </button>
        <button
          class="primary"
          @click="onSave"
        >
          保存
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.36);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(2px);
  z-index: 20;
}
.dialog {
  width: 430px;
  max-width: calc(100vw - 24px);
  max-height: calc(100vh - 32px);
  overflow-y: auto;
  background: var(--bb-surface);
  padding: 16px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-lg);
  box-shadow: var(--bb-shadow-dialog);
  display: flex;
  flex-direction: column;
  gap: 9px;
}
.dialog h3 {
  margin: 0 0 4px;
  color: var(--bb-text);
  font-size: 16px;
}
input,
textarea,
select {
  width: 100%;
  min-width: 0;
  padding: 7px 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  font-size: 14px;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 4px;
}
.primary {
  border-color: var(--bb-primary);
  background: var(--bb-primary);
  color: #fff;
  font-weight: 600;
}
.primary:hover {
  border-color: var(--bb-primary-strong);
  background: var(--bb-primary-strong);
}
</style>

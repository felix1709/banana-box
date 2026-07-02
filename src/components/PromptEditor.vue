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
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
}
.dialog {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  width: 420px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
input,
textarea,
select {
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.primary {
  font-weight: bold;
}
</style>

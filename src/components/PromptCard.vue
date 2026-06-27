<script setup lang="ts">
import { ref, watchEffect } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes } from '@/lib/ipc'
import type { Prompt } from '@/types'

const props = defineProps<{ prompt: Prompt }>()
const lib = useLibraryStore()
const ui = useUiStore()
const url = ref('')

watchEffect(async () => {
  if (props.prompt.image) {
    try {
      url.value = await readImageBytes(props.prompt.image)
    } catch {
      url.value = ''
    }
  }
})

async function onCopy() {
  await lib.copyPrompt(props.prompt.id)
  ui.showToast('已复制 ✓')
}

function onPreview() {
  if (props.prompt.image) ui.preview(props.prompt.image)
}
</script>

<template>
  <div
    class="card"
    @click="onCopy"
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
</style>

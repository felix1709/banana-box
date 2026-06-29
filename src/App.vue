<script setup lang="ts">
import { onMounted, ref, watchEffect } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { readImageBytes } from '@/lib/ipc'
import SearchBar from '@/components/SearchBar.vue'
import CategoryTree from '@/components/CategoryTree.vue'
import PromptCard from '@/components/PromptCard.vue'
import PromptEditor from '@/components/PromptEditor.vue'
import SettingsModal from '@/components/SettingsModal.vue'

const lib = useLibraryStore()
const ui = useUiStore()
const previewUrl = ref('')
const expandedPromptId = ref<string | null>(null)

onMounted(async () => {
  await lib.load()
  ui.showPanel()
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
  >
    <header class="topbar">
      <SearchBar />
      <button
        class="btn"
        @click="ui.openSettings()"
      >
        ⚙️
      </button>
      <button
        class="btn primary"
        @click="ui.openEditor(null)"
      >
        ＋
      </button>
    </header>
    <div class="body">
      <aside class="sidebar">
        <CategoryTree />
      </aside>
      <main class="content">
        <PromptCard
          v-for="p in lib.filteredPrompts"
          :key="p.id"
          :prompt="p"
          :expanded="expandedPromptId === p.id"
          @expand="expandedPromptId = $event"
        />
        <p
          v-if="lib.filteredPrompts.length === 0"
          class="empty"
        >
          未找到匹配的提示词
        </p>
      </main>
    </div>
    <PromptEditor v-if="ui.editorOpen" />
    <SettingsModal v-if="ui.settingsOpen" />
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
  gap: 8px;
  padding: 8px;
  border-bottom: 1px solid #eee;
  background: #f8fafc;
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
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
  background: #fff;
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

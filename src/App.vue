<script setup lang="ts">
import { onMounted } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import SearchBar from '@/components/SearchBar.vue'
import CategoryTree from '@/components/CategoryTree.vue'
import PromptCard from '@/components/PromptCard.vue'
import PromptEditor from '@/components/PromptEditor.vue'
import SettingsModal from '@/components/SettingsModal.vue'

const lib = useLibraryStore()
const ui = useUiStore()

onMounted(async () => {
  await lib.load()
  ui.showPanel()
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
  </div>
</template>

<style scoped>
.app {
  width: 720px;
  height: 520px;
  display: flex;
  flex-direction: column;
  font-family: system-ui, sans-serif;
}
.topbar {
  display: flex;
  gap: 8px;
  padding: 8px;
  border-bottom: 1px solid #eee;
}
.body {
  flex: 1;
  display: flex;
  overflow: hidden;
}
.sidebar {
  width: 160px;
  border-right: 1px solid #eee;
  overflow-y: auto;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
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
</style>

<script setup lang="ts">
import { useUiStore, type ActiveTool } from '@/stores/ui'
import CategoryTree from '@/components/CategoryTree.vue'

const ui = useUiStore()

const tools: { id: ActiveTool; label: string }[] = [
  { id: 'prompts', label: '提示词库' },
  { id: 'reverse-image', label: '反推图片' },
  { id: 'compression', label: '快速压缩' },
]
</script>

<template>
  <nav class="app-sidebar">
    <template
      v-for="tool in tools"
      :key="tool.id"
    >
      <div
        v-if="tool.id === 'prompts'"
        class="tool-row"
        :data-tool-row="tool.id"
      >
        <button
          type="button"
          class="tool-button"
          :class="{ active: ui.activeTool === tool.id }"
          :data-tool="tool.id"
          :aria-expanded="ui.activeTool === 'prompts'"
          @click="ui.setActiveTool(tool.id)"
        >
          {{ tool.label }}
        </button>
        <button
          type="button"
          class="create-prompt-button"
          data-action="create-prompt"
          aria-label="Create prompt"
          title="Create prompt"
          @click.stop="ui.openEditor(null)"
        >
          +
        </button>
      </div>
      <button
        v-else
        type="button"
        class="tool-button"
        :class="{ active: ui.activeTool === tool.id }"
        :data-tool="tool.id"
        :data-tool-row="tool.id"
        @click="ui.setActiveTool(tool.id)"
      >
        {{ tool.label }}
      </button>
      <div
        v-if="tool.id === 'prompts' && ui.activeTool === 'prompts'"
        class="sidebar-category-list"
      >
        <CategoryTree compact />
      </div>
    </template>
  </nav>
</template>

<style scoped>
.app-sidebar {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  height: 100%;
  min-height: 0;
}

.tool-row {
  display: flex;
  gap: 6px;
  align-items: stretch;
}

.tool-button {
  width: 100%;
  min-height: 32px;
  padding: 6px 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  background: transparent;
  color: #334155;
  cursor: pointer;
  font-size: 13px;
  text-align: left;
}

.tool-row .tool-button {
  flex: 1 1 auto;
  min-width: 0;
}

.create-prompt-button {
  width: 32px;
  min-height: 32px;
  flex: 0 0 32px;
  border: 1px solid #fecaca;
  border-radius: 6px;
  background: #fee2e2;
  color: #b91c1c;
  cursor: pointer;
  font-size: 20px;
  font-weight: 700;
  line-height: 1;
}

.create-prompt-button:hover {
  border-color: #fca5a5;
  background: #fecaca;
}

.create-prompt-button:focus-visible {
  outline: 2px solid #ef4444;
  outline-offset: 2px;
}

.tool-button:hover {
  background: #f1f5f9;
}

.tool-button.active {
  border-color: #cbd5e1;
  background: #eaf0f7;
  color: #0f172a;
  font-weight: 600;
}

.sidebar-category-list {
  max-height: 300px;
  min-height: 48px;
  overflow-y: auto;
  overflow-x: hidden;
  border-left: 2px solid #e2e8f0;
  margin: -2px 0 2px 8px;
  padding-left: 4px;
  scrollbar-gutter: stable;
}
</style>

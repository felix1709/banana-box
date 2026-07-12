<script setup lang="ts">
import { useUiStore, type ActiveTool } from '@/stores/ui'
import CategoryTree from '@/components/CategoryTree.vue'

const ui = useUiStore()

const tools: { id: ActiveTool; label: string }[] = [
  { id: 'prompts', label: '提示词库' },
  { id: 'reverse-image', label: '反推图片' },
  { id: 'compression', label: '快速压缩' },
  { id: 'projects', label: '项目管理' },
  { id: 'daily-tasks', label: '当日任务' },
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
  padding: 9px 8px;
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
  padding: 6px 9px;
  border: 1px solid transparent;
  border-radius: var(--bb-radius-md);
  background: transparent;
  color: var(--bb-text-muted);
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  text-align: left;
  box-shadow: none;
}

.tool-row .tool-button {
  flex: 1 1 auto;
  min-width: 0;
}

.create-prompt-button {
  width: 32px;
  min-height: 32px;
  flex: 0 0 32px;
  border: 1px solid rgba(102, 247, 211, 0.55);
  border-radius: var(--bb-radius-md);
  background: linear-gradient(180deg, var(--bb-primary-strong), var(--bb-primary));
  color: #041017;
  cursor: pointer;
  font-size: 20px;
  font-weight: 700;
  line-height: 1;
  box-shadow: 0 0 22px rgba(102, 247, 211, 0.2);
}

.create-prompt-button:hover {
  border-color: var(--bb-primary-strong);
  background: linear-gradient(180deg, #c2fff2, #78ffdf);
}

.create-prompt-button:focus-visible {
  outline: none;
  box-shadow: var(--bb-focus);
}

.tool-button:hover {
  border-color: rgba(123, 255, 226, 0.16);
  background: rgba(102, 247, 211, 0.07);
  color: var(--bb-text);
}

.tool-button.active {
  border-color: rgba(123, 255, 226, 0.32);
  background:
    linear-gradient(135deg, rgba(102, 247, 211, 0.18), rgba(82, 157, 255, 0.08));
  color: var(--bb-text);
  font-weight: 600;
  box-shadow:
    inset 0 0 0 1px rgba(102, 247, 211, 0.04),
    0 0 26px rgba(102, 247, 211, 0.08);
}

.sidebar-category-list {
  max-height: 300px;
  min-height: 48px;
  overflow-y: auto;
  overflow-x: hidden;
  border-left: 1px solid rgba(102, 247, 211, 0.14);
  margin: -1px 0 2px 9px;
  padding: 3px 0 3px 5px;
  scrollbar-gutter: stable;
}
</style>

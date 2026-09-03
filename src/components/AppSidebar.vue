<script setup lang="ts">
import { ref, type Component } from 'vue'
import {
  Archive,
  Bookmark,
  CalendarDays,
  ChevronDown,
  ChevronRight,
  FileText,
  FolderKanban,
  Globe,
  Image as ImageIcon,
  Library,
  Plus,
  Video,
} from '@lucide/vue'
import { useUiStore, type ActiveTool } from '@/stores/ui'
import { useAtlasStore } from '@/stores/atlas'
import CategoryTree from '@/components/CategoryTree.vue'
import { ATLAS_DIMENSIONS } from '@/lib/atlas-dimensions'

const ui = useUiStore()
const atlas = useAtlasStore()
const promptCategoriesExpanded = ref(false)
const atlasCategoriesExpanded = ref(false)

interface ToolItem {
  id: ActiveTool
  label: string
  icon: Component
}

const tools: ToolItem[] = [
  { id: 'shared-library', label: '共享图库', icon: Library },
  { id: 'prompts', label: '提示词库', icon: FileText },
  { id: 'reverse-image', label: '反推图片', icon: ImageIcon },
  { id: 'compression', label: '快速压缩', icon: Archive },
  { id: 'depth-video', label: '深度视频', icon: Video },
  { id: 'projects', label: '项目管理', icon: FolderKanban },
  { id: 'daily-tasks', label: '当日任务', icon: CalendarDays },
  { id: 'pi-web', label: '网页服务', icon: Globe },
  { id: 'atlas', label: '审美参考', icon: Bookmark },
]

function selectTool(toolId: ActiveTool) {
  if (toolId === 'prompts') {
    ui.setActiveTool('prompts')
    promptCategoriesExpanded.value = !promptCategoriesExpanded.value
    atlasCategoriesExpanded.value = false
    return
  }

  if (toolId === 'atlas') {
    ui.setActiveTool('atlas')
    atlasCategoriesExpanded.value = !atlasCategoriesExpanded.value
    promptCategoriesExpanded.value = false
    return
  }

  promptCategoriesExpanded.value = false
  atlasCategoriesExpanded.value = false
  ui.setActiveTool(toolId)
}

function togglePromptCategories() {
  promptCategoriesExpanded.value = !promptCategoriesExpanded.value
  atlasCategoriesExpanded.value = false
  ui.setActiveTool('prompts')
}

function toggleAtlasCategories() {
  atlasCategoriesExpanded.value = !atlasCategoriesExpanded.value
  promptCategoriesExpanded.value = false
  ui.setActiveTool('atlas')
}

function selectAtlasDimension(dimension: string | null) {
  atlas.dimension = dimension
  ui.setActiveTool('atlas')
}
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
          :aria-expanded="promptCategoriesExpanded"
          @click="selectTool(tool.id)"
        >
          <component
            :is="tool.icon"
            :size="15"
            class="tool-icon"
            aria-hidden="true"
          />
          <span>{{ tool.label }}</span>
        </button>
        <button
          type="button"
          class="category-toggle-button"
          :data-category-toggle="tool.id"
          :aria-expanded="promptCategoriesExpanded"
          :title="promptCategoriesExpanded ? '收起分类' : '展开分类'"
          @click.stop="togglePromptCategories"
        >
          <ChevronDown
            v-if="promptCategoriesExpanded"
            :size="14"
            aria-hidden="true"
          />
          <ChevronRight
            v-else
            :size="14"
            aria-hidden="true"
          />
        </button>
      </div>
      <div
        v-else-if="tool.id === 'atlas'"
        class="tool-row"
        :data-tool-row="tool.id"
      >
        <button
          type="button"
          class="tool-button"
          :class="{ active: ui.activeTool === tool.id }"
          :data-tool="tool.id"
          :aria-expanded="atlasCategoriesExpanded"
          @click="selectTool(tool.id)"
        >
          <component
            :is="tool.icon"
            :size="15"
            class="tool-icon"
            aria-hidden="true"
          />
          <span>{{ tool.label }}</span>
        </button>
        <button
          type="button"
          class="category-toggle-button"
          data-category-toggle="atlas"
          :aria-expanded="atlasCategoriesExpanded"
          :title="atlasCategoriesExpanded ? '收起分类' : '展开分类'"
          @click.stop="toggleAtlasCategories"
        >
          <ChevronDown
            v-if="atlasCategoriesExpanded"
            :size="14"
            aria-hidden="true"
          />
          <ChevronRight
            v-else
            :size="14"
            aria-hidden="true"
          />
        </button>
      </div>
      <button
        v-else
        type="button"
        class="tool-button"
        :class="{ active: ui.activeTool === tool.id }"
        :data-tool="tool.id"
        :data-tool-row="tool.id"
        @click="selectTool(tool.id)"
      >
        <component
          :is="tool.icon"
          :size="15"
          class="tool-icon"
          aria-hidden="true"
        />
        <span>{{ tool.label }}</span>
      </button>

      <div
        v-if="tool.id === 'prompts' && ui.activeTool === 'prompts' && promptCategoriesExpanded"
        class="sidebar-category-list"
      >
        <button
          type="button"
          class="create-prompt-header"
          data-action="create-prompt"
          aria-label="新增提示词"
          title="新增提示词"
          @click.stop="ui.openEditor(null)"
        >
          <Plus
            :size="15"
            class="create-prompt-plus"
            aria-hidden="true"
          />
          <span>新建提示词</span>
        </button>
        <CategoryTree compact />
      </div>
      <div
        v-if="tool.id === 'atlas' && ui.activeTool === 'atlas' && atlasCategoriesExpanded"
        class="sidebar-category-list atlas-category-list"
      >
        <button
          type="button"
          class="atlas-category-button"
          :class="{ active: atlas.dimension === null }"
          @click="selectAtlasDimension(null)"
        >
          全部
        </button>
        <button
          v-for="dimension in ATLAS_DIMENSIONS"
          :key="dimension.id"
          type="button"
          class="atlas-category-button"
          :class="{ active: atlas.dimension === dimension.id }"
          @click="selectAtlasDimension(dimension.id)"
        >
          {{ dimension.label }}
        </button>
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
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 7px;
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

.tool-icon {
  flex: 0 0 auto;
  color: currentColor;
}

.category-toggle-button {
  display: grid;
  width: 28px;
  min-height: 34px;
  flex: 0 0 28px;
  place-items: center;
  padding: 0;
  border: 1px solid rgba(102, 247, 211, 0.26);
  border-radius: var(--bb-radius-md);
  background: transparent;
  color: var(--bb-text-muted);
  cursor: pointer;
  box-shadow: none;
}

.category-toggle-button:hover,
.category-toggle-button:focus-visible {
  border-color: var(--bb-primary-strong);
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
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
  display: block;
  max-height: none;
  overflow: visible;
  margin: 0;
  padding: 0;
  scrollbar-gutter: stable;
}

.create-prompt-header {
  width: 100%;
  min-height: 34px;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 7px 9px;
  border: 1px solid var(--bb-danger-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-danger-soft);
  color: var(--bb-danger);
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  margin-bottom: 6px;
}

.create-prompt-header:hover,
.create-prompt-header:focus-visible {
  background: rgba(255, 107, 122, 0.2);
  border-color: var(--bb-danger);
}

.create-prompt-plus {
  flex: 0 0 auto;
  color: var(--bb-danger);
}

.atlas-category-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.atlas-category-button {
  width: 100%;
  min-height: 30px;
  padding: 5px 9px;
  border: 1px solid transparent;
  border-radius: var(--bb-radius-sm);
  background: transparent;
  color: var(--bb-text-muted);
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.atlas-category-button:hover,
.atlas-category-button:focus-visible {
  background: rgba(102, 247, 211, 0.07);
  color: var(--bb-text);
}

.atlas-category-button.active {
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
  font-weight: 600;
}
</style>

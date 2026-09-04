<script setup lang="ts">
import { ref, type Component } from 'vue'
import {
  Archive,
  Bookmark,
  CalendarDays,
  Check,
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderKanban,
  Globe,
  LayoutGrid,
  Library,
  Pencil,
  Plus,
  Trash2,
  Video,
  X,
} from '@lucide/vue'
import { useUiStore, type ActiveTool } from '@/stores/ui'
import { useAtlasStore } from '@/stores/atlas'
import CategoryTree from '@/components/CategoryTree.vue'

const ui = useUiStore()
const atlas = useAtlasStore()
const promptCategoriesExpanded = ref(false)
const atlasCategoriesExpanded = ref(false)
const atlasCategoryCreating = ref(false)
const atlasCategoryEditingId = ref<string | null>(null)
const atlasCategoryDraft = ref('')
const atlasCategoryBusy = ref(false)

interface ToolItem {
  id: ActiveTool
  label: string
  icon: Component
}

const tools: ToolItem[] = [
  { id: 'shared-library', label: '共享词库', icon: Library },
  { id: 'prompts', label: '提示词库', icon: FileText },
  { id: 'compression', label: '快速压缩', icon: Archive },
  { id: 'depth-video', label: '深度视频', icon: Video },
  { id: 'projects', label: '项目管理', icon: FolderKanban },
  { id: 'daily-tasks', label: '当日任务', icon: CalendarDays },
  { id: 'pi-web', label: 'Pi智能体', icon: Globe },
  { id: 'atlas', label: '参考图库', icon: Bookmark },
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

function selectAtlasCategory(id: string | null) {
  atlas.selectCategory(id)
  ui.setActiveTool('atlas')
}

function startCreateAtlasCategory() {
  atlasCategoryEditingId.value = null
  atlasCategoryDraft.value = ''
  atlasCategoryCreating.value = true
}

function startEditAtlasCategory(id: string, name: string) {
  atlasCategoryCreating.value = false
  atlasCategoryEditingId.value = id
  atlasCategoryDraft.value = name
}

function cancelAtlasCategoryEdit() {
  atlasCategoryCreating.value = false
  atlasCategoryEditingId.value = null
  atlasCategoryDraft.value = ''
}

async function submitCreateAtlasCategory() {
  if (atlasCategoryBusy.value) return
  atlasCategoryBusy.value = true
  try {
    await atlas.createCategory(atlasCategoryDraft.value)
    ui.showToast('分类已创建')
    cancelAtlasCategoryEdit()
  } catch (error) {
    ui.showToast(atlasCategoryError(error))
  } finally {
    atlasCategoryBusy.value = false
  }
}

async function submitRenameAtlasCategory() {
  if (!atlasCategoryEditingId.value || atlasCategoryBusy.value) return
  atlasCategoryBusy.value = true
  try {
    await atlas.renameCategory(atlasCategoryEditingId.value, atlasCategoryDraft.value)
    ui.showToast('分类已重命名')
    cancelAtlasCategoryEdit()
  } catch (error) {
    ui.showToast(atlasCategoryError(error))
  } finally {
    atlasCategoryBusy.value = false
  }
}

async function removeAtlasCategory(id: string, name: string) {
  if (!window.confirm(`删除分类「${name}」后，只会解除图片关联，不会删除图片。确定删除吗？`)) {
    return
  }
  atlasCategoryBusy.value = true
  try {
    await atlas.deleteCategory(id)
    ui.showToast('分类已删除')
  } catch (error) {
    ui.showToast(atlasCategoryError(error))
  } finally {
    atlasCategoryBusy.value = false
  }
}

function atlasCategoryError(error: unknown) {
  const message = String(error)
  if (message.includes('ATLAS_CATEGORY_NAME_EMPTY')) return '分类名称不能为空'
  if (message.includes('ATLAS_CATEGORY_DUPLICATE')) return '已存在同名分类'
  if (message.includes('ATLAS_CATEGORY_NAME_TOO_LONG')) return '分类名称不能超过 40 个字'
  return '分类操作失败，请重试'
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
          :data-category-toggle="tool.id"
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
          class="create-prompt-header"
          data-action="create-atlas-category"
          aria-label="新建分类"
          title="新建分类"
          @click.stop="startCreateAtlasCategory"
        >
          <Plus
            :size="15"
            class="create-prompt-plus"
            aria-hidden="true"
          />
          <span>新建分类</span>
        </button>

        <button
          type="button"
          class="sidebar-category-main sidebar-category-all"
          :class="{ active: atlas.selectedCategoryId === null }"
          @click="selectAtlasCategory(null)"
        >
          <LayoutGrid :size="14" />
          <span>全部参考</span>
          <small>{{ atlas.entries.length }}</small>
        </button>

        <form
          v-if="atlasCategoryCreating"
          class="sidebar-category-editor"
          @submit.prevent="submitCreateAtlasCategory"
        >
          <input
            v-model="atlasCategoryDraft"
            autofocus
            maxlength="40"
            placeholder="输入分类名称"
            aria-label="新分类名称"
          >
          <button
            type="submit"
            title="保存"
            aria-label="保存新分类"
          >
            <Check :size="13" />
          </button>
          <button
            type="button"
            title="取消"
            aria-label="取消新建分类"
            @click="cancelAtlasCategoryEdit"
          >
            <X :size="13" />
          </button>
        </form>

        <div
          v-for="category in atlas.categories"
          :key="category.id"
          class="sidebar-category-row"
          :class="{ active: atlas.selectedCategoryId === category.id }"
        >
          <button
            type="button"
            class="sidebar-category-main"
            :title="category.name"
            @click="selectAtlasCategory(category.id)"
          >
            <Folder :size="14" />
            <span>{{ category.name }}</span>
            <small>{{ category.entryIds.length }}</small>
          </button>
          <div
            v-if="atlas.selectedCategoryId === category.id"
            class="sidebar-category-actions"
          >
            <button
              type="button"
              class="sidebar-category-action"
              title="重命名"
              aria-label="重命名分类"
              @click="startEditAtlasCategory(category.id, category.name)"
            >
              <Pencil :size="13" />
            </button>
            <button
              type="button"
              class="sidebar-category-action danger"
              title="删除分类"
              aria-label="删除分类"
              @click="removeAtlasCategory(category.id, category.name)"
            >
              <Trash2 :size="13" />
            </button>
          </div>

          <form
            v-if="atlasCategoryEditingId === category.id"
            class="sidebar-category-editor"
            @submit.prevent="submitRenameAtlasCategory"
          >
            <input
              v-model="atlasCategoryDraft"
              autofocus
              maxlength="40"
              placeholder="输入分类名称"
              aria-label="分类新名称"
            >
            <button
              type="submit"
              title="保存"
              aria-label="保存重命名"
            >
              <Check :size="13" />
            </button>
            <button
              type="button"
              title="取消"
              aria-label="取消重命名"
              @click="cancelAtlasCategoryEdit"
            >
              <X :size="13" />
            </button>
          </form>
        </div>
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

.sidebar-category-main {
  width: 100%;
  min-height: 30px;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border: 1px solid transparent;
  border-radius: var(--bb-radius-sm);
  background: transparent;
  color: var(--bb-text-muted);
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}

.sidebar-category-main span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-category-main small {
  color: var(--bb-text-muted);
}

.sidebar-category-main:hover,
.sidebar-category-main:focus-visible {
  background: rgba(102, 247, 211, 0.07);
  color: var(--bb-text);
}

.sidebar-category-row.active .sidebar-category-main {
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
  font-weight: 600;
}

.sidebar-category-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px;
}

.sidebar-category-row > .sidebar-category-main {
  flex: 1 1 auto;
  min-width: 0;
}

.sidebar-category-actions {
  display: flex;
  align-items: center;
}

.sidebar-category-action {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--bb-radius-sm);
  background: transparent;
  color: var(--bb-text-muted);
  cursor: pointer;
}

.sidebar-category-action:hover,
.sidebar-category-action:focus-visible {
  background: rgba(102, 247, 211, 0.07);
  color: var(--bb-text);
}

.sidebar-category-action.danger {
  color: var(--bb-danger, #ff6b6b);
}

.sidebar-category-editor {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 0;
  flex-basis: 100%;
}

.sidebar-category-editor input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text);
  padding: 5px 6px;
  font-size: 12px;
}

.sidebar-category-editor button {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
}

</style>

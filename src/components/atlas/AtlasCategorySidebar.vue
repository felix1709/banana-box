<script setup lang="ts">
import { ref } from 'vue'
import { Check, Folder, LayoutGrid, Pencil, Plus, Trash2, X } from '@lucide/vue'
import { useAtlasStore } from '@/stores/atlas'
import { useUiStore } from '@/stores/ui'

const atlas = useAtlasStore()
const ui = useUiStore()

const creating = ref(false)
const editingId = ref<string | null>(null)
const draft = ref('')
const busy = ref(false)

function startCreate() {
  editingId.value = null
  draft.value = ''
  creating.value = true
}

function startEdit(id: string, name: string) {
  creating.value = false
  editingId.value = id
  draft.value = name
}

function cancelEdit() {
  creating.value = false
  editingId.value = null
  draft.value = ''
}

async function submitCreate() {
  if (busy.value) return
  busy.value = true
  try {
    await atlas.createCategory(draft.value)
    ui.showToast('分类已创建')
    cancelEdit()
  } catch (error) {
    ui.showToast(categoryError(error))
  } finally {
    busy.value = false
  }
}

async function submitRename() {
  if (!editingId.value || busy.value) return
  busy.value = true
  try {
    await atlas.renameCategory(editingId.value, draft.value)
    ui.showToast('分类已重命名')
    cancelEdit()
  } catch (error) {
    ui.showToast(categoryError(error))
  } finally {
    busy.value = false
  }
}

async function removeCategory(id: string, name: string) {
  if (!window.confirm(`删除分类「${name}」后，只会解除图片关联，不会删除图片。确定删除吗？`)) {
    return
  }
  busy.value = true
  try {
    await atlas.deleteCategory(id)
    ui.showToast('分类已删除')
  } catch (error) {
    ui.showToast(categoryError(error))
  } finally {
    busy.value = false
  }
}

function categoryError(error: unknown) {
  const message = String(error)
  if (message.includes('ATLAS_CATEGORY_NAME_EMPTY')) return '分类名称不能为空'
  if (message.includes('ATLAS_CATEGORY_DUPLICATE')) return '已存在同名分类'
  if (message.includes('ATLAS_CATEGORY_NAME_TOO_LONG')) return '分类名称不能超过 40 个字'
  return '分类操作失败，请重试'
}
</script>

<template>
  <aside class="atlas-category-sidebar">
    <header class="atlas-category-header">
      <h2>分类</h2>
      <button
        type="button"
        class="atlas-category-create"
        data-action="create-atlas-category"
        @click="startCreate"
      >
        <Plus :size="14" />
        <span>新建分类</span>
      </button>
    </header>

    <button
      type="button"
      class="atlas-category-row atlas-category-all"
      :class="{ active: atlas.selectedCategoryId === null }"
      @click="atlas.selectCategory(null)"
    >
      <LayoutGrid :size="14" />
      <span>全部参考</span>
      <small>{{ atlas.entries.length }}</small>
    </button>

    <form
      v-if="creating"
      class="atlas-category-editor"
      @submit.prevent="submitCreate"
    >
      <input
        v-model="draft"
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
        <Check :size="14" />
      </button>
      <button
        type="button"
        title="取消"
        aria-label="取消新建分类"
        @click="cancelEdit"
      >
        <X :size="14" />
      </button>
    </form>

    <div
      v-for="category in atlas.categories"
      :key="category.id"
      class="atlas-category-row"
      :class="{ active: atlas.selectedCategoryId === category.id }"
    >
      <button
        type="button"
        class="atlas-category-main"
        :title="category.name"
        @click="atlas.selectCategory(category.id)"
      >
        <Folder :size="14" />
        <span>{{ category.name }}</span>
        <small>{{ category.entryIds.length }}</small>
      </button>
      <div class="atlas-category-actions">
        <button
          type="button"
          title="重命名"
          aria-label="重命名分类"
          @click="startEdit(category.id, category.name)"
        >
          <Pencil :size="13" />
        </button>
        <button
          type="button"
          class="danger"
          title="删除分类"
          aria-label="删除分类"
          @click="removeCategory(category.id, category.name)"
        >
          <Trash2 :size="13" />
        </button>
      </div>

      <form
        v-if="editingId === category.id"
        class="atlas-category-editor"
        @submit.prevent="submitRename"
      >
        <input
          v-model="draft"
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
          <Check :size="14" />
        </button>
        <button
          type="button"
          title="取消"
          aria-label="取消重命名"
          @click="cancelEdit"
        >
          <X :size="14" />
        </button>
      </form>
    </div>
  </aside>
</template>

<style scoped>
.atlas-category-sidebar {
  width: 170px;
  flex: 0 0 170px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  border-right: 1px solid var(--bb-border);
  background: var(--bb-surface-soft);
  overflow-y: auto;
  min-height: 0;
}

.atlas-category-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}

.atlas-category-header h2 {
  margin: 0;
  font-size: 13px;
}

.atlas-category-create,
.atlas-category-row button,
.atlas-category-editor button {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text);
  cursor: pointer;
}

.atlas-category-create {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 6px;
  color: var(--bb-primary);
  border-color: var(--bb-primary);
}

.atlas-category-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 2px;
  border: 1px solid transparent;
  border-radius: var(--bb-radius-sm);
}

.atlas-category-row.active {
  border-color: var(--bb-primary);
  background: var(--bb-primary-soft);
}

.atlas-category-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px;
  text-align: left;
}

.atlas-category-main span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.atlas-category-row small {
  color: var(--bb-text-muted);
}

.atlas-category-actions {
  display: flex;
  align-items: center;
}

.atlas-category-actions button {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
}

.atlas-category-actions .danger {
  color: var(--bb-danger, #ff6b6b);
}

.atlas-category-editor {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 0;
  flex-basis: 100%;
}

.atlas-category-editor input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text);
  padding: 5px 6px;
}

.atlas-category-editor button {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  padding: 0;
}
</style>

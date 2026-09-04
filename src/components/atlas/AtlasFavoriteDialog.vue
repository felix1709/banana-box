<script setup lang="ts">
import { ref } from 'vue'
import { Check, Heart, Plus, X } from '@lucide/vue'
import type { AtlasEntry } from '@/types/atlas'
import { useAtlasStore } from '@/stores/atlas'
import { useUiStore } from '@/stores/ui'

const props = defineProps<{
  entry: AtlasEntry
  imageUrl: string
}>()

const emit = defineEmits<{
  close: []
  saved: []
}>()

const atlas = useAtlasStore()
const ui = useUiStore()
const selectedIds = ref<string[]>([...props.entry.categoryIds])
const newCategoryName = ref('')
const creating = ref(false)
const saving = ref(false)

function toggleCategory(id: string) {
  selectedIds.value = selectedIds.value.includes(id)
    ? selectedIds.value.filter((item) => item !== id)
    : [...selectedIds.value, id]
}

async function createCategory() {
  if (creating.value) return
  creating.value = true
  try {
    await atlas.createCategory(newCategoryName.value)
    const created = atlas.categories[atlas.categories.length - 1]
    if (created && !selectedIds.value.includes(created.id)) {
      selectedIds.value = [...selectedIds.value, created.id]
    }
    newCategoryName.value = ''
  } catch (error) {
    ui.showToast(categoryError(error))
  } finally {
    creating.value = false
  }
}

async function save() {
  if (saving.value) return
  saving.value = true
  try {
    await atlas.setEntryCategories(props.entry.id, selectedIds.value)
    ui.showToast('收藏分类已更新')
    emit('saved')
  } catch (error) {
    ui.showToast(categoryError(error))
  } finally {
    saving.value = false
  }
}

function categoryError(error: unknown) {
  const message = String(error)
  if (message.includes('ATLAS_CATEGORY_NAME_EMPTY')) return '分类名称不能为空'
  if (message.includes('ATLAS_CATEGORY_DUPLICATE')) return '已存在同名分类'
  if (message.includes('ATLAS_CATEGORY_NAME_TOO_LONG')) return '分类名称不能超过 40 个字'
  return '收藏操作失败，请重试'
}
</script>

<template>
  <div
    class="atlas-favorite-overlay"
    @click.self="emit('close')"
  >
    <section
      class="atlas-favorite-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="选择收藏分类"
    >
      <header class="atlas-favorite-header">
        <div>
          <Heart
            :size="16"
            class="heart-icon"
          />
          <strong>收藏到分类</strong>
        </div>
        <button
          type="button"
          aria-label="关闭"
          title="关闭"
          @click="emit('close')"
        >
          <X :size="16" />
        </button>
      </header>

      <div class="atlas-favorite-entry">
        <img
          v-if="props.imageUrl"
          :src="props.imageUrl"
          alt=""
        >
        <strong>{{ props.entry.title }}</strong>
      </div>

      <div class="atlas-favorite-categories">
        <p
          v-if="atlas.categories.length === 0"
          class="atlas-favorite-empty"
        >
          还没有分类，先创建一个项目分类吧
        </p>
        <label
          v-for="category in atlas.categories"
          :key="category.id"
          class="atlas-favorite-category"
        >
          <input
            type="checkbox"
            :checked="selectedIds.includes(category.id)"
            @change="toggleCategory(category.id)"
          >
          <span>{{ category.name }}</span>
          <small>{{ category.entryIds.length }}</small>
        </label>
      </div>

      <form
        class="atlas-favorite-create"
        @submit.prevent="createCategory"
      >
        <input
          v-model="newCategoryName"
          maxlength="40"
          placeholder="新建分类名称"
          aria-label="新建分类名称"
        >
        <button
          type="submit"
          :disabled="creating"
        >
          <Plus :size="14" />
          新建
        </button>
      </form>

      <footer class="atlas-favorite-actions">
        <button
          type="button"
          class="secondary"
          @click="emit('close')"
        >
          取消
        </button>
        <button
          type="button"
          class="primary"
          :disabled="saving"
          @click="save"
        >
          <Check :size="14" />
          保存收藏
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.atlas-favorite-overlay {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(3px);
}

.atlas-favorite-dialog {
  width: min(420px, calc(100vw - 32px));
  max-height: min(560px, calc(100vh - 32px));
  overflow-y: auto;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  color: var(--bb-text);
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.35);
  padding: 12px;
}

.atlas-favorite-header,
.atlas-favorite-entry,
.atlas-favorite-create,
.atlas-favorite-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.atlas-favorite-header {
  justify-content: space-between;
  margin-bottom: 10px;
}

.atlas-favorite-header > div {
  display: flex;
  align-items: center;
  gap: 6px;
}

.atlas-favorite-header button,
.atlas-favorite-actions button,
.atlas-favorite-create button {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
  padding: 6px 8px;
}

.heart-icon {
  color: var(--bb-danger, #ff6b6b);
  fill: currentColor;
}

.atlas-favorite-entry {
  gap: 10px;
  padding-bottom: 10px;
  border-bottom: 1px solid var(--bb-border);
  margin-bottom: 10px;
}

.atlas-favorite-entry img {
  width: 56px;
  height: 56px;
  object-fit: cover;
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
}

.atlas-favorite-categories {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
}

.atlas-favorite-category {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  cursor: pointer;
}

.atlas-favorite-category span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.atlas-favorite-category small {
  color: var(--bb-text-muted);
}

.atlas-favorite-empty {
  margin: 0;
  padding: 12px;
  color: var(--bb-text-muted);
  text-align: center;
}

.atlas-favorite-create {
  margin-top: 10px;
}

.atlas-favorite-create input {
  flex: 1;
  min-width: 0;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  padding: 6px 8px;
}

.atlas-favorite-create button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.atlas-favorite-actions {
  justify-content: flex-end;
  margin-top: 12px;
}

.atlas-favorite-actions .primary {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #081014;
  background: var(--bb-primary);
  border-color: var(--bb-primary);
  font-weight: 600;
}
</style>

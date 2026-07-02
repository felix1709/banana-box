<script setup lang="ts">
import { ref } from 'vue'
import { FAVORITES_CATEGORY_ID, useLibraryStore } from '@/stores/library'
import CategoryDialog from '@/components/CategoryDialog.vue'

withDefaults(defineProps<{ compact?: boolean }>(), {
  compact: false,
})

const lib = useLibraryStore()
const showDialog = ref(false)

function onAdd() {
  showDialog.value = true
}

function onConfirm(payload: { name: string; color: string }) {
  lib.addCategory(payload.name, payload.color)
  showDialog.value = false
}
</script>

<template>
  <div
    class="tree"
    :class="{ compact }"
  >
    <div
      class="item"
      :class="{ active: lib.currentCategoryId === null }"
      @click="lib.currentCategoryId = null"
    >
      全部
    </div>
    <div
      class="item favorite-category"
      :class="{ active: lib.currentCategoryId === FAVORITES_CATEGORY_ID }"
      @click="lib.currentCategoryId = FAVORITES_CATEGORY_ID"
    >
      <span class="favorite-dot">★</span>
      收藏
    </div>
    <div
      v-for="c in lib.categories"
      :key="c.id"
      class="item"
      :class="{ active: lib.currentCategoryId === c.id }"
      @click="lib.currentCategoryId = c.id"
    >
      <span
        class="dot"
        :style="{ background: c.color }"
      />
      {{ c.name }}
      <button
        class="del"
        @click.stop="lib.deleteCategory(c.id)"
      >
        ×
      </button>
    </div>
    <button
      class="add"
      @click="onAdd"
    >
      ＋ 新建分类
    </button>
    <CategoryDialog
      :visible="showDialog"
      @confirm="onConfirm"
      @close="showDialog = false"
    />
  </div>
</template>

<style scoped>
.tree {
  padding: 8px;
}
.tree.compact {
  padding: 4px 0 4px 4px;
  font-size: 11px;
  line-height: 1.25;
}
.item {
  padding: 6px 8px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.tree.compact .item {
  min-height: 24px;
  padding: 4px 5px;
  border-radius: 5px;
}
.item:hover,
.item.active {
  background: #f0f0f0;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.favorite-dot {
  width: 12px;
  flex: 0 0 12px;
  color: #d97706;
  font-size: 12px;
  line-height: 1;
}
.del {
  margin-left: auto;
  border: none;
  background: none;
  cursor: pointer;
  color: #c00;
  font-size: 11px;
  line-height: 1;
}
.add {
  width: 100%;
  margin-top: 8px;
  padding: 6px;
  cursor: pointer;
}
.tree.compact .add {
  min-height: 24px;
  margin-top: 5px;
  padding: 4px 5px;
  font-size: 11px;
}
</style>

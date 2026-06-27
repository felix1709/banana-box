<script setup lang="ts">
import { useLibraryStore } from '@/stores/library'

const lib = useLibraryStore()

function onAdd() {
  const name = window.prompt('分类名称')
  if (name) lib.addCategory(name)
}
</script>

<template>
  <div class="tree">
    <div
      class="item"
      :class="{ active: lib.currentCategoryId === null }"
      @click="lib.currentCategoryId = null"
    >
      全部
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
  </div>
</template>

<style scoped>
.tree {
  padding: 8px;
}
.item {
  padding: 6px 8px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  gap: 6px;
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
.del {
  margin-left: auto;
  border: none;
  background: none;
  cursor: pointer;
  color: #c00;
}
.add {
  width: 100%;
  margin-top: 8px;
  padding: 6px;
  cursor: pointer;
}
</style>

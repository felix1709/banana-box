<script setup lang="ts">
import { useLibraryStore } from '@/stores/library'

const lib = useLibraryStore()
</script>

<template>
  <input
    v-model="lib.search"
    class="search"
    type="text"
    placeholder="🔍 搜索提示词、标签..."
  >
</template>

<style scoped>
.search {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
}
</style>

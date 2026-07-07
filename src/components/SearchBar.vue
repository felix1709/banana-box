<script setup lang="ts">
import { useLibraryStore } from '@/stores/library'

const lib = useLibraryStore()
</script>

<template>
  <input
    v-model="lib.search"
    class="search"
    type="text"
    placeholder="搜索提示词、标签..."
  >
</template>

<style scoped>
.search {
  flex: 1;
  min-height: 32px;
  padding: 6px 11px;
  border: 1px solid rgba(123, 255, 226, 0.18);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(180deg, rgba(12, 25, 36, 0.92), rgba(5, 14, 22, 0.92)),
    var(--bb-surface);
  color: var(--bb-text);
  font-size: 14px;
  box-shadow:
    inset 0 0 22px rgba(102, 247, 211, 0.04),
    var(--bb-shadow-sm);
}

.search:focus {
  background: rgba(5, 14, 22, 0.96);
}
</style>

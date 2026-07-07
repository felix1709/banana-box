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
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.96)),
    var(--bb-surface);
  color: var(--bb-text);
  font-size: 14px;
  box-shadow: var(--bb-shadow-sm);
}

.search:focus {
  background: var(--bb-surface);
}
</style>

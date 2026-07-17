<script setup lang="ts">
import { onMounted } from 'vue'
import { RefreshCw } from '@lucide/vue'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'
import { useUiStore } from '@/stores/ui'
import SharedPromptCard from '@/components/shared/SharedPromptCard.vue'
import type { SharedPrompt } from '@/types'

const shared = useSharedLibraryStore()
const ui = useUiStore()

onMounted(() => {
  void shared.load()
})

async function copyPrompt(prompt: SharedPrompt) {
  await shared.copySharedPrompt(prompt.id)
  ui.showToast('已复制共享提示词')
}

function downloadPrompt(prompt: SharedPrompt) {
  shared.downloadToLocal(prompt)
  ui.showToast('已加入本地提示词库')
}
</script>

<template>
  <section class="shared-library-page">
    <header class="shared-library-toolbar">
      <div>
        <p>Shared</p>
        <h2>共享库</h2>
      </div>
      <button
        class="shared-refresh-button"
        type="button"
        data-action="refresh-shared-library"
        :disabled="shared.loading"
        @click="shared.load"
      >
        <RefreshCw
          :size="15"
          aria-hidden="true"
        />
        {{ shared.loading ? '读取中' : '刷新' }}
      </button>
      <input
        v-model="shared.search"
        class="shared-library-search"
        data-field="shared-library-search"
        type="search"
        placeholder="搜索共享提示词"
        aria-label="搜索共享提示词"
      >
    </header>

    <p
      v-if="shared.error"
      class="shared-library-error"
      role="alert"
    >
      {{ shared.error }}
    </p>

    <div class="shared-library-list scrollable-panel">
      <SharedPromptCard
        v-for="prompt in shared.filteredPrompts"
        :key="prompt.id"
        :prompt="prompt"
        @copy="copyPrompt"
        @download="downloadPrompt"
      />

      <p
        v-if="!shared.loading && shared.filteredPrompts.length === 0"
        class="shared-library-empty"
      >
        暂无共享提示词
      </p>
    </div>
  </section>
</template>

<style scoped>
.shared-library-page {
  display: flex;
  min-height: 0;
  height: 100%;
  flex-direction: column;
  background: var(--bb-bg);
}

.shared-library-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  justify-content: space-between;
  gap: 12px;
  padding: 20px 22px 16px;
  border-bottom: 1px solid var(--bb-border);
}

.shared-library-toolbar p,
.shared-library-toolbar h2 {
  margin: 0;
}

.shared-library-toolbar p {
  color: var(--bb-primary);
  font-family: var(--bb-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.shared-library-toolbar h2 {
  margin-top: 3px;
  font-size: 20px;
  font-weight: 650;
}

.shared-refresh-button {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  font-size: 12px;
}

.shared-library-search {
  width: min(220px, 100%);
  height: 30px;
  padding: 0 9px;
}

.shared-library-error {
  margin: 0;
  padding: 8px 22px;
  border-bottom: 1px solid var(--bb-danger-border);
  background: var(--bb-danger-soft);
  color: #ffb6c0;
}

.shared-library-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 14px;
}

.shared-library-empty {
  margin: 32px 0;
  color: var(--bb-text-soft);
  text-align: center;
}
</style>

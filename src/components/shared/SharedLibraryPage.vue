<script setup lang="ts">
import { onMounted } from 'vue'
import { Copy, Download, RefreshCw } from '@lucide/vue'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'
import { useUiStore } from '@/stores/ui'
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
      <article
        v-for="prompt in shared.filteredPrompts"
        :key="prompt.id"
        class="shared-prompt-card"
        :data-shared-prompt-card="prompt.id"
        tabindex="0"
        @dblclick="copyPrompt(prompt)"
        @keydown.enter.prevent="copyPrompt(prompt)"
      >
        <div class="shared-prompt-main">
          <strong>{{ prompt.title }}</strong>
          <p>{{ prompt.content }}</p>
          <div class="shared-prompt-tags">
            <span
              v-for="tag in prompt.tags"
              :key="tag"
            >
              {{ tag }}
            </span>
          </div>
          <small>{{ prompt.createdByName || '共享用户' }}</small>
          <span
            v-if="shared.hasLocalReference(prompt.id)"
            class="shared-prompt-downloaded"
          >
            已下载
          </span>
        </div>

        <div class="shared-prompt-actions">
          <button
            type="button"
            title="复制"
            aria-label="复制"
            data-action="copy-shared-prompt"
            :data-shared-prompt-id="prompt.id"
            @click.stop="copyPrompt(prompt)"
          >
            <Copy
              :size="14"
              aria-hidden="true"
            />
          </button>
          <button
            type="button"
            title="下载到本地"
            aria-label="下载到本地"
            data-action="download-shared-prompt"
            :data-shared-prompt-id="prompt.id"
            :disabled="shared.hasLocalReference(prompt.id)"
            @click.stop="downloadPrompt(prompt)"
          >
            <Download
              :size="14"
              aria-hidden="true"
            />
          </button>
        </div>
      </article>

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
  display: grid;
  align-content: start;
  gap: 10px;
  min-height: 0;
  flex: 1;
  padding: 14px;
}

.shared-prompt-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 10px;
  min-height: 104px;
  padding: 11px;
  border: 1px solid rgba(148, 179, 188, 0.18);
  border-radius: var(--bb-radius-sm);
  background:
    linear-gradient(180deg, rgba(20, 35, 47, 0.96), rgba(9, 21, 31, 0.96));
  box-shadow: var(--bb-shadow-card);
  cursor: pointer;
}

.shared-prompt-card:hover,
.shared-prompt-card:focus-visible {
  border-color: rgba(123, 255, 226, 0.42);
  outline: none;
  box-shadow: var(--bb-shadow-floating);
}

.shared-prompt-main {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.shared-prompt-main strong {
  overflow: hidden;
  color: var(--bb-text);
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shared-prompt-main p {
  display: -webkit-box;
  height: 4.05em;
  margin: 0;
  overflow: hidden;
  color: var(--bb-text-muted);
  font-size: 11px;
  line-height: 1.35;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.shared-prompt-main small {
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.shared-prompt-downloaded {
  justify-self: start;
  padding: 2px 7px;
  border: 1px solid rgba(123, 255, 226, 0.3);
  border-radius: var(--bb-radius-xs);
  background: rgba(123, 255, 226, 0.08);
  color: var(--bb-primary);
  font-size: 10px;
  font-weight: 650;
}

.shared-prompt-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  min-width: 0;
}

.shared-prompt-tags span {
  max-width: 100%;
  overflow: hidden;
  padding: 1px 6px;
  border: 1px solid rgba(102, 247, 211, 0.16);
  border-radius: 999px;
  background: rgba(102, 247, 211, 0.08);
  color: #b5d3dc;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.shared-prompt-actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.shared-prompt-actions button {
  display: grid;
  width: 28px;
  min-height: 28px;
  place-items: center;
  padding: 0;
}

.shared-library-empty {
  margin: 32px 0;
  color: var(--bb-text-soft);
  text-align: center;
}
</style>

<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue'
import { Copy, Download, Trash2 } from '@lucide/vue'
import { readImageBytes } from '@/lib/ipc'
import { useAuthStore } from '@/stores/auth'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'
import { useUiStore } from '@/stores/ui'
import type { SharedPrompt } from '@/types'

const props = defineProps<{ prompt: SharedPrompt }>()
const emit = defineEmits<{
  copy: [prompt: SharedPrompt]
  download: [prompt: SharedPrompt]
}>()

const auth = useAuthStore()
const shared = useSharedLibraryStore()
const ui = useUiStore()
const expanded = ref(false)
const favorite = ref(false)
const imageUrl = ref('')
const deleting = ref(false)
const isAdmin = computed(() => auth.isCloudAdmin)

watchEffect(async () => {
  if (!props.prompt.image) {
    imageUrl.value = ''
    return
  }
  try {
    imageUrl.value = await readImageBytes(props.prompt.image)
  } catch {
    imageUrl.value = ''
  }
})

function toggleExpanded(event: MouseEvent) {
  if (event.detail > 1) return
  expanded.value = !expanded.value
}

function toggleFavorite() {
  favorite.value = !favorite.value
}

function previewImage() {
  if (props.prompt.image) ui.preview(props.prompt.image)
}

async function deletePrompt() {
  if (!isAdmin.value || deleting.value) return
  deleting.value = true
  try {
    await shared.deleteSharedPrompt(props.prompt.id)
    ui.showToast('已删除共享提示词')
  } catch (error) {
    ui.showToast(error instanceof Error ? error.message : String(error))
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <article
    class="shared-prompt-card card"
    :class="[
      expanded ? 'expanded' : 'collapsed',
      {
        'expanded-auto-height-card': expanded,
        'flow-expanded-card': expanded,
        'fixed-size-prompt-card': !expanded,
      },
    ]"
    :data-shared-prompt-card="prompt.id"
    tabindex="0"
    @click="toggleExpanded"
    @dblclick.stop="emit('copy', prompt)"
    @keydown.enter.prevent="emit('copy', prompt)"
  >
    <button
      type="button"
      class="favorite-button icon-only-star"
      :class="{ active: favorite }"
      :aria-pressed="favorite"
      title="收藏"
      aria-label="收藏"
      @click.stop="toggleFavorite"
    />

    <div
      class="card-main fixed-card-layout"
      :class="{ 'flow-card-main': expanded }"
    >
      <div
        class="text-pane"
        :class="{ 'flow-text-pane': expanded }"
      >
        <div class="title">
          {{ prompt.title }}
        </div>
        <div
          class="content fixed-three-line-content"
          :class="{ 'full-content': expanded, 'flow-content': expanded }"
        >
          {{ prompt.content }}
        </div>
        <div
          class="tags"
          :class="{ 'full-tags': expanded }"
        >
          <span
            v-for="tag in prompt.tags"
            :key="tag"
            class="tag"
            :class="{ 'full-tag': expanded }"
          >
            {{ tag }}
          </span>
        </div>
        <small class="shared-author">{{ prompt.createdByName || '共享用户' }}</small>
        <span
          v-if="shared.hasLocalReference(prompt.id)"
          class="shared-prompt-downloaded"
        >
          已下载
        </span>

        <div
          v-if="expanded"
          class="action-panel"
        >
          <div class="actions">
            <button
              type="button"
              data-action="copy-shared-prompt"
              :data-shared-prompt-id="prompt.id"
              @click.stop="emit('copy', prompt)"
            >
              <Copy
                :size="13"
                aria-hidden="true"
              />
              复制
            </button>
            <button
              type="button"
              data-action="download-shared-prompt"
              :data-shared-prompt-id="prompt.id"
              :disabled="shared.hasLocalReference(prompt.id)"
              @click.stop="emit('download', prompt)"
            >
              <Download
                :size="13"
                aria-hidden="true"
              />
              下载
            </button>
            <button
              v-if="isAdmin"
              type="button"
              class="danger"
              data-action="delete-shared-prompt"
              :data-shared-prompt-id="prompt.id"
              :disabled="deleting"
              @click.stop="deletePrompt"
            >
              <Trash2
                :size="13"
                aria-hidden="true"
              />
              删除
            </button>
          </div>
        </div>
      </div>

      <button
        type="button"
        class="thumb-zone fit-box prompt-preview-thumb stable-preview-thumb"
        :class="{ empty: !imageUrl }"
        title="预览图"
        aria-label="预览图"
        @click.stop="previewImage"
      >
        <img
          v-if="imageUrl"
          class="thumb fit-contain"
          :src="imageUrl"
          alt="参考图"
        >
        <span v-else>预览图</span>
      </button>
    </div>
  </article>
</template>

<style scoped>
.card {
  position: relative;
  box-sizing: border-box;
  height: 104px;
  min-height: 104px;
  max-height: 104px;
  max-width: 100%;
  overflow: hidden;
  padding: 7px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(180deg, rgba(20, 35, 47, 0.96), rgba(9, 21, 31, 0.96));
  box-shadow: var(--bb-shadow-card);
  cursor: pointer;
  user-select: none;
  transition:
    border-color 120ms ease,
    background-color 120ms ease,
    box-shadow 120ms ease;
}

.card.expanded {
  z-index: 2;
  height: auto;
  min-height: 104px;
  max-height: none;
  overflow: visible;
  border-color: rgba(123, 255, 226, 0.36);
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.1), transparent 38%),
    linear-gradient(180deg, rgba(23, 41, 54, 0.98), rgba(10, 22, 32, 0.98));
  box-shadow: var(--bb-shadow-floating);
}

.card.flow-expanded-card {
  height: auto;
  min-height: 104px;
  max-height: none;
  align-self: stretch;
  flex: 0 0 auto;
}

.card:hover,
.card:focus-visible {
  border-color: rgba(123, 255, 226, 0.34);
  outline: none;
  box-shadow:
    var(--bb-shadow-card),
    0 0 26px rgba(102, 247, 211, 0.08);
}

.card-main {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 96px;
  gap: 8px;
  align-items: start;
  min-height: 100%;
}

.card.expanded .card-main {
  height: auto;
  min-height: 0;
  align-items: start;
}

.card-main.flow-card-main {
  height: auto;
  min-height: auto;
  align-items: start;
  grid-auto-rows: max-content;
}

.text-pane {
  display: flex;
  min-width: 0;
  flex-direction: column;
  overflow: hidden;
}

.text-pane.flow-text-pane,
.card.expanded .text-pane {
  height: auto;
  min-height: 0;
  overflow: visible;
}

.title {
  min-width: 0;
  overflow: hidden;
  padding-right: 20px;
  color: var(--bb-text);
  font-size: 13px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.content {
  display: -webkit-box;
  height: 4.05em;
  max-height: 4.05em;
  margin: 4px 0;
  overflow: hidden;
  color: var(--bb-text-muted);
  font-size: 11px;
  line-height: 1.35;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.content.full-content,
.content.flow-content,
.card.expanded .content {
  display: block;
  height: auto;
  min-height: 0;
  max-height: none;
  overflow: visible;
  position: static;
  -webkit-line-clamp: unset;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  min-width: 0;
  max-height: 16px;
  overflow: hidden;
}

.tags.full-tags,
.card.expanded .tags {
  max-height: none;
  overflow: visible;
}

.tag {
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

.tag.full-tag,
.card.expanded .tag {
  overflow: visible;
  overflow-wrap: anywhere;
  text-overflow: clip;
  white-space: normal;
}

.shared-author {
  margin-top: 3px;
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.shared-prompt-downloaded {
  align-self: start;
  margin-top: 3px;
  padding: 2px 7px;
  border: 1px solid rgba(123, 255, 226, 0.3);
  border-radius: var(--bb-radius-xs);
  background: rgba(123, 255, 226, 0.08);
  color: var(--bb-primary);
  font-size: 10px;
  font-weight: 650;
}

.thumb-zone {
  display: flex;
  width: 96px;
  height: 72px;
  min-height: 72px;
  max-height: 72px;
  align-items: center;
  justify-content: center;
  justify-self: end;
  padding: 0;
  overflow: hidden;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(135deg, rgba(21, 38, 51, 0.92), rgba(8, 18, 27, 0.92)),
    var(--bb-surface-soft);
  color: var(--bb-text-muted);
  font-size: 11px;
}

.thumb-zone.empty {
  border-style: dashed;
}

.thumb-zone:hover {
  border-color: rgba(123, 255, 226, 0.36);
  background: var(--bb-primary-soft);
}

.thumb {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #07121b;
}

.favorite-button {
  position: absolute;
  top: 7px;
  right: 7px;
  z-index: 4;
  display: flex;
  width: 18px;
  height: 18px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--bb-text-soft);
  font-size: 0;
}

.favorite-button:hover,
.favorite-button.active {
  background: transparent;
  color: var(--bb-favorite);
}

.favorite-button::before {
  content: "★";
  font-size: 14px;
  line-height: 1;
}

.action-panel {
  position: relative;
}

.actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 5px;
  margin-top: 8px;
  padding-top: 4px;
}

.actions button {
  display: inline-flex;
  min-height: 25px;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 3px 6px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-xs);
  background: rgba(5, 14, 22, 0.74);
  color: var(--bb-text);
  cursor: pointer;
  font-size: 12px;
}

.actions button:hover:not(:disabled) {
  border-color: var(--bb-border-strong);
  background: var(--bb-surface-soft);
}

.actions button:disabled {
  cursor: default;
  opacity: 0.55;
}

.actions button.danger {
  border-color: rgba(255, 100, 122, 0.32);
  color: #ff9cab;
}
</style>

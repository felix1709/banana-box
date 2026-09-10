<script setup lang="ts">
import { computed, ref } from 'vue'
import { Copy, Heart, Languages, X } from '@lucide/vue'
import type { TemplateCase } from '@/types/template-library'
import { loadTemplateImageBytes } from '@/lib/template-library'
import { copyToClipboard, saveImage } from '@/lib/ipc'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'

const props = defineProps<{
  item: TemplateCase
  imageUrl: string
}>()

const emit = defineEmits<{
  close: []
}>()

const lib = useLibraryStore()
const ui = useUiStore()
const language = ref<'zh' | 'en'>('zh')
const saving = ref(false)

const activePrompt = computed(() =>
  language.value === 'zh'
    ? props.item.promptZh || props.item.promptEn
    : props.item.promptEn,
)

function toggleLanguage() {
  language.value = language.value === 'zh' ? 'en' : 'zh'
}

async function copyPrompt() {
  const text = activePrompt.value.trim()
  if (!text) {
    ui.showToast('当前没有可复制的内容')
    return
  }
  await copyToClipboard(text)
  ui.showToast('已复制')
}

async function favorite() {
  if (saving.value) return
  saving.value = true
  try {
    if (!lib.loaded) await lib.load()
    const title = props.item.title
    const content = activePrompt.value.trim()
    if (!content) {
      ui.showToast('当前提示词为空，无法收藏')
      return
    }
    const existing = lib.library.prompts.find(
      (prompt) => prompt.title === title && prompt.content === content,
    )
    if (existing) {
      ui.showToast('这条模板已在提示词库中')
      return
    }

    let image: string | null = null
    try {
      const bytes = await loadTemplateImageBytes(props.item.image)
      image = await saveImage(bytes, 'webp')
    } catch {
      image = null
    }

    let category = lib.categories.find((item) => item.name === '模板收藏')
    if (!category) {
      lib.addCategory('模板收藏')
      category = lib.categories[lib.categories.length - 1]
    }

    lib.addPrompt({
      title,
      content,
      categoryId: category?.id ?? null,
      tags: [props.item.category],
      image,
    })
    ui.showToast('已收藏到提示词库')
  } catch {
    ui.showToast('收藏失败，请重试')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div
    class="template-dialog-overlay"
    @click.self="emit('close')"
  >
    <section
      class="template-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="模板案例详情"
    >
      <div class="template-dialog-image">
        <img
          v-if="props.imageUrl"
          :src="props.imageUrl"
          :alt="props.item.title"
        >
        <div
          v-else
          class="template-dialog-no-image"
        >
          暂无图片
        </div>
      </div>

      <aside class="template-dialog-detail">
        <header class="template-dialog-header">
          <div>
            <h2>{{ props.item.title }}</h2>
            <p v-if="props.item.sourceLabel">
              来源：{{ props.item.sourceLabel }}
            </p>
          </div>
          <button
            type="button"
            class="template-dialog-close"
            aria-label="关闭"
            title="关闭"
            @click="emit('close')"
          >
            <X :size="18" />
          </button>
        </header>

        <div class="template-dialog-toolbar">
          <button
            type="button"
            class="template-dialog-action"
            @click="toggleLanguage"
          >
            <Languages :size="15" />
            {{ language === 'zh' ? '查看英文' : '查看中文' }}
          </button>
          <button
            type="button"
            class="template-dialog-action"
            @click="copyPrompt"
          >
            <Copy :size="15" />
            复制提示词
          </button>
          <button
            type="button"
            class="template-dialog-action"
            :disabled="saving"
            @click="favorite"
          >
            <Heart :size="15" />
            {{ saving ? '收藏中…' : '收藏到提示词库' }}
          </button>
        </div>

        <div class="template-dialog-prompt">
          <pre>{{ activePrompt }}</pre>
        </div>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.template-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(3px);
}

.template-dialog {
  width: min(920px, calc(100vw - 32px));
  height: min(620px, calc(100vh - 32px));
  display: grid;
  grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr);
  overflow: hidden;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  color: var(--bb-text);
  box-shadow: 0 18px 60px rgba(0, 0, 0, 0.42);
}

.template-dialog-image {
  min-width: 0;
  min-height: 0;
  display: grid;
  place-items: center;
  padding: 14px;
  background:
    radial-gradient(circle at 20% 0%, rgba(102, 247, 211, 0.08), transparent 40%),
    var(--bb-surface-soft);
  overflow: hidden;
}

.template-dialog-image img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: var(--bb-radius-sm);
}

.template-dialog-no-image {
  color: var(--bb-text-muted);
  font-size: 13px;
}

.template-dialog-detail {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  overflow-y: auto;
}

.template-dialog-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.template-dialog-header h2 {
  margin: 0;
  font-size: 16px;
  line-height: 1.4;
}

.template-dialog-header p {
  margin: 6px 0 0;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.template-dialog-close {
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
}

.template-dialog-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.template-dialog-action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 32px;
  padding: 6px 10px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
  font-size: 12px;
}

.template-dialog-action:hover,
.template-dialog-action:focus-visible {
  border-color: var(--bb-primary-strong);
}

.template-dialog-action:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.template-dialog-prompt {
  flex: 1;
  min-height: 0;
  padding: 12px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  overflow-y: auto;
}

.template-dialog-prompt pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--bb-text);
  font-family: inherit;
  font-size: 13px;
  line-height: 1.65;
}
</style>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Check, Copy, Heart, X } from '@lucide/vue'
import type { AtlasEntry } from '@/types/atlas'
import { analyzeAtlasEntry, loadAtlasEntry } from '@/lib/atlas-ipc'
import {
  buildGptPrompt,
  buildMjPrompt,
  type AtlasPreviewPromptMode,
} from '@/lib/atlas-preview-prompts'

const props = defineProps<{
  entry: AtlasEntry
  imageUrl: string
}>()

const emit = defineEmits<{
  close: []
  favorite: []
  analyzed: []
}>()

const markdown = ref('')
const loading = ref(true)
const analyzing = ref(false)
const analyzeError = ref('')
const loadError = ref('')
const viewMode = ref<AtlasPreviewPromptMode>('analysis')
const copyMessage = ref('')
const copyError = ref('')

const analysisBody = computed(() => stripFrontmatter(markdown.value))
const gptOutput = computed(() => buildGptPrompt(analysisBody.value))
const mjOutput = computed(() => buildMjPrompt(analysisBody.value, props.entry.tags))
const activeOutput = computed(() => {
  if (viewMode.value === 'gpt') return gptOutput.value
  if (viewMode.value === 'mj') return mjOutput.value
  return analysisBody.value
})

const sections = computed(() => {
  const result: { title: string; body: string }[] = []
  const source = stripFrontmatter(markdown.value)
  if (!source) return result

  let currentTitle = ''
  let currentBody: string[] = []
  const flush = () => {
    if (currentTitle) {
      result.push({ title: currentTitle, body: currentBody.join('\n').trim() })
    }
  }

  for (const line of source.split(/\r?\n/)) {
    if (line.startsWith('## ')) {
      flush()
      currentTitle = line.slice(3).trim()
      currentBody = []
    } else if (currentTitle) {
      currentBody.push(line)
    }
  }
  flush()
  return result.filter((section) => section.body)
})

function stripFrontmatter(source: string) {
  if (!source) return ''
  const trimmed = source.trimStart()
  if (!trimmed.startsWith('---\n')) return source
  const rest = trimmed.slice(4)
  const end = rest.indexOf('\n---')
  if (end === -1) return source
  return rest.slice(end + 4).trimStart()
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    close()
  }
}

function close() {
  viewMode.value = 'analysis'
  copyMessage.value = ''
  copyError.value = ''
  emit('close')
}

function selectMode(mode: AtlasPreviewPromptMode) {
  viewMode.value = mode
  copyMessage.value = ''
  copyError.value = ''
}

async function copyActiveOutput() {
  copyError.value = ''
  copyMessage.value = ''
  const text = activeOutput.value.trim()
  if (!text) {
    copyError.value = '当前没有可复制的内容'
    return
  }

  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
    } else {
      const textarea = document.createElement('textarea')
      textarea.value = text
      textarea.style.position = 'fixed'
      textarea.style.opacity = '0'
      document.body.appendChild(textarea)
      textarea.select()
      document.execCommand('copy')
      textarea.remove()
    }
    copyMessage.value = '已复制'
  } catch {
    copyError.value = '复制失败，请手动选择复制'
  }
}

async function analyze() {
  if (analyzing.value) return
  analyzing.value = true
  analyzeError.value = ''
  try {
    await analyzeAtlasEntry(props.entry.id)
    markdown.value = (await loadAtlasEntry(props.entry.id)) ?? ''
    emit('analyzed')
  } catch (error) {
    analyzeError.value = errorText(error)
  } finally {
    analyzing.value = false
  }
}

function errorText(error: unknown) {
  if (error instanceof Error && error.message) return error.message
  return String(error)
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  try {
    markdown.value = (await loadAtlasEntry(props.entry.id)) ?? ''
  } catch (error) {
    loadError.value = errorText(error)
    markdown.value = ''
  } finally {
    loading.value = false
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div
    class="atlas-preview-overlay"
    @click.self="close"
  >
    <section
      class="atlas-preview-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="参考图片详情"
    >
      <button
        type="button"
        class="atlas-preview-close"
        aria-label="关闭"
        title="关闭"
        @click="close"
      >
        <X :size="18" />
      </button>

      <div class="atlas-preview-image">
        <img
          v-if="props.imageUrl"
          :src="props.imageUrl"
          :alt="props.entry.title"
        >
        <div
          v-else
          class="atlas-preview-no-image"
        >
          暂无图片
        </div>
      </div>

      <aside class="atlas-preview-detail">
        <header class="atlas-preview-title">
          <button
            type="button"
            class="atlas-preview-favorite"
            :class="{ active: props.entry.categoryIds.length > 0 }"
            :aria-label="props.entry.categoryIds.length > 0 ? '修改收藏分类' : '收藏到分类'"
            :title="props.entry.categoryIds.length > 0 ? '修改收藏分类' : '收藏到分类'"
            @click="emit('favorite')"
          >
            <Heart
              :size="16"
              :fill="props.entry.categoryIds.length > 0 ? 'currentColor' : 'none'"
            />
          </button>
        </header>

        <div
          v-if="props.entry.tags.length"
          class="atlas-preview-tags"
        >
          <span
            v-for="tag in props.entry.tags"
            :key="tag"
          >
            {{ tag }}
          </span>
        </div>

        <div class="atlas-preview-tabs" role="tablist" aria-label="提示词视图">
          <button
            type="button"
            role="tab"
            :aria-selected="viewMode === 'analysis'"
            :class="{ active: viewMode === 'analysis' }"
            @click="selectMode('analysis')"
          >
            分析结果
          </button>
          <button
            type="button"
            role="tab"
            :aria-selected="viewMode === 'gpt'"
            :class="{ active: viewMode === 'gpt' }"
            @click="selectMode('gpt')"
          >
            GPT 风格
          </button>
          <button
            type="button"
            role="tab"
            :aria-selected="viewMode === 'mj'"
            :class="{ active: viewMode === 'mj' }"
            @click="selectMode('mj')"
          >
            MJ 风格
          </button>
        </div>

        <div class="atlas-preview-content">
          <div class="atlas-preview-actions">
            <button
              type="button"
              class="atlas-preview-copy"
              @click="copyActiveOutput"
            >
              <Copy :size="14" />
              {{ viewMode === 'gpt' ? '复制 GPT 风格' : viewMode === 'mj' ? '复制 MJ 风格' : '复制分析结果' }}
            </button>
            <span
              v-if="copyMessage"
              class="atlas-preview-copy-ok"
            >
              <Check :size="13" />
              {{ copyMessage }}
            </span>
            <span
              v-if="copyError"
              class="atlas-preview-copy-error"
            >
              {{ copyError }}
            </span>
          </div>

          <div
            v-if="props.entry.status === 'pending_review'"
            class="atlas-preview-analyze"
          >
            <button
              type="button"
              :disabled="analyzing"
              @click="analyze"
            >
              {{ analyzing ? '正在分析…' : '立即分析' }}
            </button>
            <span
              v-if="analyzeError"
              class="atlas-preview-analyze-error"
            >
              {{ analyzeError }}
            </span>
          </div>

          <div
            v-if="loading"
            class="atlas-preview-loading"
          >
            正在读取提示词…
          </div>

          <div
            v-else-if="loadError"
            class="atlas-preview-load-error"
          >
            读取失败：{{ loadError }}
          </div>

          <div
            v-else-if="viewMode === 'analysis'"
            class="atlas-preview-sections"
          >
            <section
              v-for="section in sections"
              :key="section.title"
            >
              <h3>{{ section.title }}</h3>
              <pre>{{ section.body }}</pre>
            </section>
            <p
              v-if="sections.length === 0"
              class="atlas-preview-empty"
            >
              该图片暂时没有提示词分析
            </p>
          </div>

          <div
            v-else
            class="atlas-preview-prompt"
          >
            <pre>{{ activeOutput || '当前还没有可展示的提示词内容' }}</pre>
          </div>
        </div>
      </aside>
    </section>
  </div>
</template>

<style scoped>
.atlas-preview-overlay {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.66);
  backdrop-filter: blur(6px);
  padding: 18px;
}

.atlas-preview-dialog {
  position: relative;
  width: min(1180px, 100%);
  height: min(760px, 100%);
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  overflow: hidden;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  color: var(--bb-text);
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
}

.atlas-preview-close {
  position: absolute;
  top: 10px;
  left: 10px;
  right: auto;
  z-index: 2;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 50%;
  background: rgba(4, 12, 18, 0.6);
  color: var(--bb-text);
  cursor: pointer;
  backdrop-filter: blur(5px);
}

.atlas-preview-image {
  display: grid;
  place-items: center;
  min-width: 0;
  min-height: 0;
  background:
    radial-gradient(circle at center, rgba(102, 247, 211, 0.08), transparent 60%),
    #050b0f;
  padding: 18px;
}

.atlas-preview-image img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  border-radius: var(--bb-radius-sm);
}

.atlas-preview-no-image {
  color: var(--bb-text-muted);
}

.atlas-preview-detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow: hidden;
  padding: 16px;
  border-left: 1px solid var(--bb-border);
  background: var(--bb-surface-soft);
}

.atlas-preview-tabs {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 6px;
  flex: 0 0 auto;
}

.atlas-preview-tabs button {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text-soft);
  padding: 7px 4px;
  cursor: pointer;
  white-space: nowrap;
  font-size: 13px;
  transition:
    border-color 0.16s ease,
    color 0.16s ease,
    background 0.16s ease;
}

.atlas-preview-tabs button:hover {
  border-color: var(--bb-accent, #66f7d3);
  color: var(--bb-text);
}

.atlas-preview-tabs button.active {
  border-color: var(--bb-accent, #66f7d3);
  background: rgba(102, 247, 211, 0.12);
  color: var(--bb-accent, #66f7d3);
  font-weight: 600;
}

.atlas-preview-content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-right: 2px;
}

.atlas-preview-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  flex: 0 0 auto;
}

.atlas-preview-copy {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text);
  padding: 5px 8px;
  cursor: pointer;
  font-size: 12px;
}

.atlas-preview-copy:hover {
  border-color: var(--bb-accent, #66f7d3);
  color: var(--bb-accent, #66f7d3);
}

.atlas-preview-copy-ok {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  color: var(--bb-accent, #66f7d3);
  font-size: 12px;
}

.atlas-preview-copy-error {
  color: var(--bb-danger, #ff6b6b);
  font-size: 12px;
}

.atlas-preview-title {
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  gap: 10px;
}

.atlas-preview-title > div {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.atlas-preview-title span {
  color: var(--bb-text-muted);
  font-size: 12px;
}

.atlas-preview-favorite {
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--bb-border);
  border-radius: 50%;
  background: var(--bb-surface);
  color: var(--bb-text-muted);
  cursor: pointer;
}

.atlas-preview-favorite.active {
  color: var(--bb-danger, #ff6b6b);
  border-color: var(--bb-danger, #ff6b6b);
}

.atlas-preview-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.atlas-preview-tags span {
  border: 1px solid var(--bb-border);
  border-radius: 999px;
  padding: 3px 7px;
  color: var(--bb-text-soft);
  font-size: 12px;
}

.atlas-preview-analyze {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.atlas-preview-analyze button {
  align-self: flex-start;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-accent, #66f7d3);
  color: #04121a;
  padding: 7px 12px;
  cursor: pointer;
}

.atlas-preview-analyze button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.atlas-preview-analyze-error {
  color: var(--bb-danger, #ff6b6b);
  font-size: 12px;
}

.atlas-preview-loading,
.atlas-preview-empty,
.atlas-preview-load-error {
  color: var(--bb-text-muted);
}

.atlas-preview-load-error {
  color: var(--bb-danger, #ff6b6b);
}

.atlas-preview-sections {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.atlas-preview-sections h3 {
  margin: 0 0 4px;
  font-size: 13px;
}

.atlas-preview-sections pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font: inherit;
  color: var(--bb-text-soft);
}

.atlas-preview-prompt pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font: inherit;
  color: var(--bb-text-soft);
  line-height: 1.55;
}
</style>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { TemplateCase, TemplateCategory } from '@/types/template-library'
import { loadTemplateImageUrl, loadTemplateLibrary } from '@/lib/template-library'
import TemplateCaseDialog from '@/components/template/TemplateCaseDialog.vue'

const loading = ref(false)
const error = ref('')
const categories = ref<TemplateCategory[]>([])
const cases = ref<TemplateCase[]>([])
const selectedCategory = ref('')
const thumbnailUrls = ref<Record<number, string>>({})
const previewCase = ref<TemplateCase | null>(null)
const previewImageUrl = ref('')
const thumbnailLoading = new Set<number>()
let thumbnailObserver: IntersectionObserver | null = null

const categoryOptions = computed(() => [
  { id: 'all', name: '全部', value: '' },
  ...categories.value,
])

const filteredCases = computed(() => {
  if (!selectedCategory.value) return cases.value
  return cases.value.filter((item) => item.category === selectedCategory.value)
})

async function refresh() {
  loading.value = true
  error.value = ''
  try {
    const library = await loadTemplateLibrary()
    categories.value = library.categories
    cases.value = library.cases
    thumbnailUrls.value = {}
    loading.value = false
    await nextTick()
    observeVisibleCards()
  } catch (err) {
    error.value = String(err)
    loading.value = false
  }
}

function observeVisibleCards() {
  thumbnailObserver?.disconnect()
  if (typeof IntersectionObserver === 'undefined') {
    for (const item of filteredCases.value) void loadThumbnail(item.id)
    return
  }

  thumbnailObserver = new IntersectionObserver(
    (records) => {
      for (const record of records) {
        if (!record.isIntersecting) continue
        const id = Number((record.target as HTMLElement).dataset.templateCardId)
        if (id) void loadThumbnail(id)
      }
    },
    {
      root: document.querySelector('.template-grid'),
      rootMargin: '160px',
      threshold: 0.01,
    },
  )

  for (const element of document.querySelectorAll<HTMLElement>('[data-template-card-id]')) {
    thumbnailObserver.observe(element)
  }
}

async function loadThumbnail(id: number) {
  const item = cases.value.find((entry) => entry.id === id)
  if (!item || thumbnailUrls.value[id] || thumbnailLoading.has(id)) return
  thumbnailLoading.add(id)
  try {
    thumbnailUrls.value[id] = await loadTemplateImageUrl(item.image)
  } catch {
    thumbnailUrls.value[id] = ''
  } finally {
    thumbnailLoading.delete(id)
  }
}

async function openPreview(item: TemplateCase) {
  previewCase.value = item
  previewImageUrl.value = thumbnailUrls.value[item.id] || ''
  try {
    previewImageUrl.value = await loadTemplateImageUrl(item.image)
  } catch {
    // 保留缩略图或空状态，避免预览直接失效。
  }
}

function closePreview() {
  previewCase.value = null
  previewImageUrl.value = ''
}

function selectCategory(value: string) {
  selectedCategory.value = value
}

watch(filteredCases, async () => {
  await nextTick()
  observeVisibleCards()
})

onMounted(() => {
  void refresh()
})

onBeforeUnmount(() => {
  thumbnailObserver?.disconnect()
  thumbnailObserver = null
})
</script>

<template>
  <section class="template-page scrollable-panel">
    <header class="template-categories">
      <button
        v-for="category in categoryOptions"
        :key="category.id"
        type="button"
        class="template-category-button"
        :class="{ active: selectedCategory === category.value }"
        @click="selectCategory(category.value)"
      >
        {{ category.name }}
      </button>
    </header>

    <div class="template-body">
      <p
        v-if="loading"
        class="template-empty"
      >
        正在加载模板案例…
      </p>
      <p
        v-else-if="error"
        class="template-empty"
      >
        {{ error }}
      </p>
      <main
        v-else
        class="template-grid"
      >
        <article
          v-for="item in filteredCases"
          :key="item.id"
          :data-template-card-id="item.id"
          tabindex="0"
          class="template-card"
          @click="openPreview(item)"
          @keydown.enter.prevent="openPreview(item)"
        >
          <img
            v-if="thumbnailUrls[item.id]"
            :src="thumbnailUrls[item.id]"
            alt=""
          >
          <div
            v-else
            class="template-card-placeholder"
          >
            加载中
          </div>
          <h3>{{ item.title }}</h3>
        </article>
        <p
          v-if="filteredCases.length === 0"
          class="template-empty"
        >
          当前分类还没有案例
        </p>
      </main>
    </div>

    <TemplateCaseDialog
      v-if="previewCase"
      :item="previewCase"
      :image-url="previewImageUrl"
      @close="closePreview"
    />
  </section>
</template>

<style scoped>
.template-page {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.template-categories {
  display: flex;
  gap: 8px;
  padding: 10px;
  border-bottom: 1px solid var(--bb-border);
  overflow-x: auto;
  overflow-y: hidden;
  flex: 0 0 auto;
  scrollbar-gutter: stable;
}

.template-category-button {
  flex: 0 0 auto;
  min-height: 30px;
  padding: 5px 12px;
  border: 1px solid var(--bb-border);
  border-radius: 999px;
  background: var(--bb-surface-soft);
  color: var(--bb-text-muted);
  cursor: pointer;
  font-size: 12px;
  white-space: nowrap;
}

.template-category-button:hover,
.template-category-button:focus-visible {
  border-color: var(--bb-primary-strong);
  color: var(--bb-text);
}

.template-category-button.active {
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary);
  color: #06231f;
  font-weight: 700;
}

.template-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  padding: 10px;
}

.template-grid {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 10px;
  align-content: start;
  scrollbar-gutter: stable;
}

.template-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
  text-align: left;
}

.template-card:hover,
.template-card:focus-visible {
  border-color: rgba(123, 255, 226, 0.32);
}

.template-card img,
.template-card-placeholder {
  width: 100%;
  aspect-ratio: 1 / 1;
  object-fit: cover;
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
}

.template-card-placeholder {
  display: grid;
  place-items: center;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.template-card h3 {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  line-height: 1.35;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.template-empty {
  grid-column: 1 / -1;
  margin: 32px 0;
  padding: 18px;
  color: var(--bb-text-muted);
  text-align: center;
  border: 1px dashed var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
}
</style>

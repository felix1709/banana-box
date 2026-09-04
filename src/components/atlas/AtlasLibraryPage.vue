<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Heart, Trash2 } from '@lucide/vue'
import { useAtlasStore } from '@/stores/atlas'
import AtlasFavoriteDialog from '@/components/atlas/AtlasFavoriteDialog.vue'
import AtlasImagePreviewDialog from '@/components/atlas/AtlasImagePreviewDialog.vue'
import {
  analyzeAtlasEntry,
  getAtlasServiceStatus,
  loadAtlasImage,
  loadAtlasThumbnail,
  openAtlasFolder,
  startAtlasService,
  stopAtlasService,
} from '@/lib/atlas-ipc'

const atlas = useAtlasStore()
const thumbnailUrls = ref<Record<string, string>>({})
const previewImageUrl = ref('')
const serviceRunning = ref(false)
const serviceBusy = ref(false)
const favoriteEntryId = ref<string | null>(null)
const previewEntryId = ref<string | null>(null)
let refreshTimer: number | undefined
let thumbnailObserver: IntersectionObserver | null = null
const analysisAttempted = new Set<string>()
const thumbnailLoading = new Set<string>()

const favoriteEntry = computed(() =>
  atlas.entries.find((entry) => entry.id === favoriteEntryId.value) ?? null,
)

const previewEntry = computed(() =>
  atlas.entries.find((entry) => entry.id === previewEntryId.value) ?? null,
)

async function refreshEntries(reloadExisting = true) {
  await atlas.load()
  if (reloadExisting) {
    thumbnailUrls.value = {}
  }
  await nextTick()
  observeVisibleCards()
  void analyzePendingEntries()
}

function observeVisibleCards() {
  thumbnailObserver?.disconnect()
  if (typeof IntersectionObserver === 'undefined') {
    for (const entry of atlas.filteredEntries) {
      void loadThumbnail(entry.id)
    }
    return
  }

  thumbnailObserver = new IntersectionObserver(
    (records) => {
      for (const record of records) {
        if (!record.isIntersecting) continue
        const id = (record.target as HTMLElement).dataset.atlasCardId
        if (id) void loadThumbnail(id)
      }
    },
    {
      root: document.querySelector('.atlas-grid'),
      rootMargin: '120px',
      threshold: 0.01,
    },
  )

  for (const element of document.querySelectorAll<HTMLElement>('[data-atlas-card-id]')) {
    thumbnailObserver.observe(element)
  }
}

async function loadThumbnail(id: string) {
  if (thumbnailUrls.value[id] || thumbnailLoading.has(id)) return
  thumbnailLoading.add(id)
  try {
    thumbnailUrls.value[id] = await loadAtlasThumbnail(id)
  } catch {
    try {
      thumbnailUrls.value[id] = await loadAtlasImage(id)
    } catch {
      thumbnailUrls.value[id] = ''
    }
  } finally {
    thumbnailLoading.delete(id)
  }
}

async function loadPreviewImage(id: string) {
  previewImageUrl.value = thumbnailUrls.value[id] || ''
  try {
    previewImageUrl.value = await loadAtlasImage(id)
  } catch {
    // 保留缩略图或空状态，避免预览直接失效。
  }
}

async function analyzePendingEntries() {
  const pendingEntries = atlas.entries.filter(
    (entry) => entry.status === 'pending_review' && !analysisAttempted.has(entry.id),
  )
  await Promise.all(
    pendingEntries.map(async (entry) => {
      analysisAttempted.add(entry.id)
      try {
        await analyzeAtlasEntry(entry.id)
        await refreshEntries(false)
      } catch {
        // 识图失败时保留 pending_review，用户仍可手动刷新或重新入库
      }
    }),
  )
}

function openPreview(id: string) {
  previewEntryId.value = id
  void loadPreviewImage(id)
}

function closePreview() {
  previewEntryId.value = null
  previewImageUrl.value = ''
}

async function handlePreviewAnalyzed() {
  await atlas.load()
}

function openPreviewFavorite() {
  if (previewEntryId.value) {
    openFavorite(previewEntryId.value)
  }
}

function startAutoRefresh() {
  window.clearInterval(refreshTimer)
  refreshTimer = window.setInterval(() => {
    refreshEntries(false).catch(() => {})
  }, 3000)
}

function stopAutoRefresh() {
  window.clearInterval(refreshTimer)
}

watch(serviceRunning, (running) => {
  if (running) {
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
})

watch(
  () => atlas.filteredEntries.map((entry) => entry.id).join(','),
  async () => {
    await nextTick()
    observeVisibleCards()
  },
)

function openFavorite(id: string) {
  favoriteEntryId.value = id
}

function closeFavorite() {
  favoriteEntryId.value = null
}

async function deleteEntry(id: string) {
  if (!window.confirm('确定删除这张参考图吗？原图文件也会一并删除。')) return
  await atlas.deleteEntry(id)
  if (previewEntryId.value === id) {
    previewEntryId.value = null
    previewImageUrl.value = ''
  }
  thumbnailUrls.value[id] = ''
}

async function toggleService() {
  if (serviceBusy.value) return
  serviceBusy.value = true
  try {
    if (serviceRunning.value) {
      await stopAtlasService()
    } else {
      await startAtlasService()
    }
    serviceRunning.value = await getAtlasServiceStatus()
  } finally {
    serviceBusy.value = false
  }
}

onMounted(async () => {
  await refreshEntries()
  serviceRunning.value = await getAtlasServiceStatus()
  if (serviceRunning.value) {
    startAutoRefresh()
  }
})

onBeforeUnmount(() => {
  stopAutoRefresh()
  thumbnailObserver?.disconnect()
  thumbnailObserver = null
})
</script>

<template>
  <section class="atlas-page scrollable-panel">
    <header class="atlas-toolbar">
      <button
        type="button"
        @click="openAtlasFolder"
      >
        打开知识库文件夹
      </button>
      <button
        type="button"
        @click="refreshEntries(true)"
      >
        刷新
      </button>
      <button
        type="button"
        :disabled="serviceBusy"
        @click="toggleService"
      >
        {{ serviceRunning ? '关闭入库服务' : '开启入库服务' }}
      </button>
      <input
        v-model="atlas.search"
        placeholder="搜索标题或标签"
      >
    </header>

    <div class="atlas-body atlas-body-with-categories">
      <div class="atlas-content">
        <main class="atlas-grid">
          <article
            v-for="entry in atlas.filteredEntries"
            :key="entry.id"
            :data-atlas-card-id="entry.id"
            tabindex="0"
            class="atlas-card"
            @click="openPreview(entry.id)"
            @keydown.enter.prevent="openPreview(entry.id)"
          >
            <img
              v-if="thumbnailUrls[entry.id]"
              :src="thumbnailUrls[entry.id]"
              alt=""
            >
            <div
              v-else
              class="atlas-card-placeholder"
            >
              暂无图片
            </div>
            <button
              type="button"
              class="atlas-card-favorite"
              :class="{ active: entry.categoryIds.length > 0 }"
              :title="entry.categoryIds.length > 0 ? '修改收藏分类' : '收藏到分类'"
              :aria-label="entry.categoryIds.length > 0 ? '修改收藏分类' : '收藏到分类'"
              @click.stop="openFavorite(entry.id)"
            >
              <Heart
                :size="15"
                :fill="entry.categoryIds.length > 0 ? 'currentColor' : 'none'"
              />
            </button>
            <button
              type="button"
              class="atlas-card-delete"
              aria-label="删除参考图"
              title="删除参考图"
              @click.stop="deleteEntry(entry.id)"
            >
              <Trash2 :size="15" />
            </button>
          </article>
        </main>
      </div>
    </div>
    <AtlasImagePreviewDialog
      v-if="previewEntry"
      :entry="previewEntry"
      :image-url="previewImageUrl"
      @close="closePreview"
      @favorite="openPreviewFavorite"
      @analyzed="handlePreviewAnalyzed"
    />
    <AtlasFavoriteDialog
      v-if="favoriteEntry"
      :entry="favoriteEntry"
      :image-url="thumbnailUrls[favoriteEntry.id] || ''"
      @close="closeFavorite"
      @saved="closeFavorite"
    />
  </section>
</template>

<style scoped>
.atlas-page {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.atlas-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 10px;
  border-bottom: 1px solid var(--bb-border);
}

.atlas-toolbar button,
.atlas-card {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  cursor: pointer;
}

.atlas-toolbar input {
  flex: 1 1 180px;
  min-width: 140px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  padding: 6px 8px;
}

.atlas-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 10px;
  min-height: 0;
  overflow: hidden;
}

.atlas-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.atlas-grid {
  overflow-y: auto;
  min-height: 0;
  flex: 1;
}

.atlas-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
  align-content: start;
}

.atlas-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 6px;
  text-align: left;
  cursor: pointer;
}

.atlas-card img {
  width: 100%;
  aspect-ratio: 1 / 1;
  object-fit: cover;
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
}

.atlas-card-placeholder {
  aspect-ratio: 1 / 1;
  display: grid;
  place-items: center;
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface);
  color: var(--bb-text-muted);
  font-size: 12px;
}

.atlas-card-favorite {
  position: absolute;
  top: 8px;
  right: 8px;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 50%;
  background: rgba(4, 12, 18, 0.62);
  color: var(--bb-text-muted);
  cursor: pointer;
  backdrop-filter: blur(5px);
}

.atlas-card-favorite.active {
  color: var(--bb-danger, #ff6b6b);
  border-color: var(--bb-danger, #ff6b6b);
}

.atlas-card-delete {
  position: absolute;
  top: 8px;
  right: 40px;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 50%;
  background: rgba(4, 12, 18, 0.62);
  color: var(--bb-text-muted);
  cursor: pointer;
  backdrop-filter: blur(5px);
}

.atlas-card-delete:hover {
  color: var(--bb-danger, #ff6b6b);
  border-color: var(--bb-danger, #ff6b6b);
}

</style>

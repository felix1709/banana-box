<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useAtlasStore } from '@/stores/atlas'
import {
  getAtlasServiceStatus,
  loadAtlasEntry,
  loadAtlasImage,
  openAtlasFolder,
  startAtlasService,
  stopAtlasService,
} from '@/lib/atlas-ipc'

const atlas = useAtlasStore()
const imageUrls = ref<Record<string, string>>({})
const selectedMarkdown = ref('')
const serviceRunning = ref(false)
const serviceBusy = ref(false)

const dimensions = [
  'scene-concept',
  'style',
  'character-pose',
  'composition',
  'lighting-atmosphere',
  'fx-effects',
  'color-texture',
  'emotion-mood',
  'camera-lens',
  'model-constraints',
]

const selectedSections = computed(() => {
  const sections: { title: string; body: string }[] = []
  if (!selectedMarkdown.value) return sections

  let currentTitle = '提示词'
  let currentBody: string[] = []
  const flush = () => {
    if (currentTitle || currentBody.length) {
      sections.push({ title: currentTitle, body: currentBody.join('\n').trim() })
    }
  }

  for (const line of selectedMarkdown.value.split(/\r?\n/)) {
    if (line.startsWith('## ')) {
      flush()
      currentTitle = line.slice(3).trim()
      currentBody = []
    } else {
      currentBody.push(line)
    }
  }
  flush()
  return sections.filter((section) => section.title !== '分析' || section.body)
})

async function refreshEntries() {
  await atlas.load()
  imageUrls.value = {}
  await Promise.all(
    atlas.entries
      .filter((entry) => entry.image)
      .map(async (entry) => {
        try {
          imageUrls.value[entry.id] = await loadAtlasImage(entry.id)
        } catch {
          imageUrls.value[entry.id] = ''
        }
      }),
  )
}

async function selectEntry(id: string) {
  atlas.select(id)
  selectedMarkdown.value = (await loadAtlasEntry(id)) ?? ''
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
})
</script>

<template>
  <section class="atlas-page scrollable-panel">
    <header class="atlas-toolbar">
      <button type="button" @click="openAtlasFolder">打开知识库文件夹</button>
      <button type="button" @click="refreshEntries">刷新</button>
      <button type="button" :disabled="serviceBusy" @click="toggleService">
        {{ serviceRunning ? '关闭入库服务' : '开启入库服务' }}
      </button>
      <input v-model="atlas.search" placeholder="搜索标题或标签" />
    </header>

    <div class="atlas-body">
      <aside class="atlas-sidebar">
        <button type="button" @click="atlas.dimension = null">全部</button>
        <button
          v-for="dimension in dimensions"
          :key="dimension"
          type="button"
          @click="atlas.dimension = dimension"
        >
          {{ dimension }}
        </button>
      </aside>

      <main class="atlas-grid">
        <button
          v-for="entry in atlas.filteredEntries"
          :key="entry.id"
          type="button"
          class="atlas-card"
          @click="selectEntry(entry.id)"
        >
          <img v-if="imageUrls[entry.id]" :src="imageUrls[entry.id]" alt="" />
          <div v-else class="atlas-card-placeholder">暂无图片</div>
          <strong>{{ entry.title }}</strong>
          <span>{{ entry.status }}</span>
        </button>
        <p v-if="atlas.filteredEntries.length === 0" class="atlas-empty">暂无参考条目</p>
      </main>

      <aside class="atlas-detail">
        <template v-if="atlas.selectedEntry">
          <img
            v-if="imageUrls[atlas.selectedEntry.id]"
            :src="imageUrls[atlas.selectedEntry.id]"
            alt=""
          />
          <div class="atlas-detail-head">
            <strong>{{ atlas.selectedEntry.title }}</strong>
            <span>{{ atlas.selectedEntry.status }}</span>
          </div>
          <div class="atlas-prompt-sections">
            <section v-for="section in selectedSections" :key="section.title">
              <h3>{{ section.title }}</h3>
              <pre>{{ section.body }}</pre>
            </section>
          </div>
        </template>
        <p v-else class="atlas-empty">点击左侧卡片查看提示词</p>
      </aside>
    </div>
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
.atlas-sidebar button,
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
  display: grid;
  grid-template-columns: 168px minmax(0, 1fr) minmax(220px, 360px);
  gap: 10px;
  padding: 10px;
  min-height: 0;
  overflow: hidden;
}

.atlas-sidebar,
.atlas-grid,
.atlas-detail {
  overflow-y: auto;
  min-height: 0;
}

.atlas-sidebar {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.atlas-sidebar button {
  text-align: left;
  padding: 7px 9px;
}

.atlas-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
  align-content: start;
}

.atlas-card {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 6px;
  text-align: left;
}

.atlas-card img,
.atlas-detail img {
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

.atlas-detail {
  padding: 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
}

.atlas-detail-head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin: 8px 0;
}

.atlas-prompt-sections {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.atlas-prompt-sections h3 {
  margin: 0 0 4px;
  font-size: 13px;
}

.atlas-prompt-sections pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font: inherit;
  color: var(--bb-text-soft);
}

.atlas-empty {
  color: var(--bb-text-muted);
  text-align: center;
  padding: 16px;
}
</style>

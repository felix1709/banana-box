<script setup lang="ts">
import { ref } from 'vue'
import { v4 as uuid } from 'uuid'
import { open } from '@tauri-apps/plugin-dialog'
import {
  DEFAULT_REVERSE_MODEL,
  DEFAULT_REVERSE_MODELS,
  useLibraryStore,
} from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import {
  checkApiConnection,
  exportLibrary,
  importLibrary,
  readImportDir,
  downloadImage,
} from '@/lib/ipc'
import { checkAppUpdate, installAppUpdate } from '@/lib/updater'
import { parseFile } from '@/lib/parse'
import type { Prompt } from '@/types'
import type { AppUpdateResult } from '@/lib/updater'

const lib = useLibraryStore()
const ui = useUiStore()
const hotkey = ref(lib.library.settings.hotkey)
const apiBaseUrl = ref(lib.library.settings.apiBaseUrl)
const apiKey = ref(lib.library.settings.apiKey)
const reverseModel = ref(lib.library.settings.reverseModel)
const availableReverseModels = ref([...lib.library.settings.availableReverseModels])
const checkingApi = ref(false)
const apiStatus = ref('')
const importing = ref(false)
const checkingVersion = ref(false)
const installingUpdate = ref(false)
const updateResult = ref<AppUpdateResult | null>(null)
const updateError = ref('')

function saveHotkey() {
  lib.library.settings.hotkey = hotkey.value
  lib.persist()
  ui.showToast('已保存')
}

function pickPreferredModel(models: string[]) {
  if (models.includes(DEFAULT_REVERSE_MODEL)) return DEFAULT_REVERSE_MODEL
  if (models.includes(reverseModel.value)) return reverseModel.value
  return models[0] ?? DEFAULT_REVERSE_MODEL
}

function saveApiSettings() {
  lib.library.settings.apiBaseUrl = apiBaseUrl.value.trim()
  lib.library.settings.apiKey = apiKey.value.trim()
  lib.library.settings.reverseModel = reverseModel.value
  lib.library.settings.availableReverseModels = [...availableReverseModels.value]
  lib.persist()
}

async function onCheckApiConnection() {
  checkingApi.value = true
  apiStatus.value = ''
  try {
    const result = await checkApiConnection({
      baseUrl: apiBaseUrl.value.trim(),
      apiKey: apiKey.value.trim(),
    })
    const models = result.models.length ? result.models : DEFAULT_REVERSE_MODELS
    availableReverseModels.value = models
    reverseModel.value = pickPreferredModel(models)
    apiStatus.value = result.message || (result.ok ? '连接成功' : '连接失败')
    saveApiSettings()
  } catch {
    availableReverseModels.value = DEFAULT_REVERSE_MODELS
    reverseModel.value = pickPreferredModel(DEFAULT_REVERSE_MODELS)
    apiStatus.value = '连接失败，已保留默认模型'
    saveApiSettings()
  } finally {
    checkingApi.value = false
  }
}

async function onExport() {
  await exportLibrary()
  ui.showToast('已导出')
}

async function onImport() {
  const imported = await importLibrary()
  if (!imported) return
  lib.hydrate(imported)
  ui.showToast('已导入')
}

// 批量导入：选目录 → 解析 .md/.txt → 下载飞书图 → 合并到当前库
async function onBatchImport() {
  const dir = await open({ directory: true, multiple: false })
  if (!dir || Array.isArray(dir)) return
  importing.value = true
  ui.showToast('解析中…')
  try {
    const files = await readImportDir(dir as string)
    const parsed = files.map((f) => parseFile(f.filename, f.content))
    const cats = [...lib.library.categories]
    const prompts: Prompt[] = []
    const pending: { prompt: Prompt; url: string }[] = []
    const now = Math.floor(Date.now() / 1000)
    const colors = [
      '#ef4444',
      '#f59e0b',
      '#facc15',
      '#22c55e',
      '#3b82f6',
      '#8b5cf6',
      '#ec4899',
      '#6b7280',
    ]
    parsed.forEach((pf, i) => {
      const catId = uuid()
      cats.push({
        id: catId,
        name: pf.category,
        color: colors[i % colors.length],
        order: cats.length,
      })
      pf.prompts.forEach((pp) => {
        const p: Prompt = {
          id: uuid(),
          title: pp.title,
          content: pp.content,
          categoryId: catId,
          tags: pp.tags,
          image: null,
          favorite: false,
          order: lib.library.prompts.length + prompts.length,
          createdAt: now,
          updatedAt: now,
        }
        prompts.push(p)
        if (pp.imageUrl) pending.push({ prompt: p, url: pp.imageUrl })
      })
    })

    let done = 0
    for (const item of pending) {
      try {
        item.prompt.image = await downloadImage(item.url)
      } catch {
        // 忽略单张失败
      }
      done++
      ui.showToast(`下载图片 ${done}/${pending.length}`)
    }

    lib.library.categories = cats
    lib.library.prompts.push(...prompts)
    lib.persist()
    ui.showToast(`导入 ${prompts.length} 条 / 图 ${pending.length}`)
  } catch {
    ui.showToast('导入失败')
  } finally {
    importing.value = false
  }
}

async function onCheckUpdate() {
  checkingVersion.value = true
  updateError.value = ''
  updateResult.value = null
  try {
    updateResult.value = await checkAppUpdate()
  } catch {
    updateError.value = '检查失败，请确认网络连接后重试'
  } finally {
    checkingVersion.value = false
  }
}

async function onDownloadUpdate() {
  installingUpdate.value = true
  updateError.value = ''
  try {
    await installAppUpdate()
  } catch {
    updateError.value = '鏇存柊瀹夎澶辫触锛岃绋嶅悗閲嶈瘯'
  } finally {
    installingUpdate.value = false
  }
}
</script>

<template>
  <div
    class="mask"
    @click.self="ui.closeSettings()"
  >
    <div class="dialog scrollable-dialog">
      <h3>设置</h3>
      <label>
        全局快捷键
        <input v-model="hotkey">
      </label>
      <button @click="saveHotkey">
        保存快捷键
      </button>
      <section class="api-panel">
        <div class="api-header">
          <strong>API 调用</strong>
          <p>填写 OpenAI 兼容接口，用于反推图片提示词。</p>
        </div>
        <label>
          Base URL
          <input
            v-model="apiBaseUrl"
            class="api-base-url-input"
            placeholder="https://ai.leihuo.netease.com"
          >
        </label>
        <label>
          API Key
          <input
            v-model="apiKey"
            class="api-key-input"
            type="password"
            placeholder="请输入 API Key"
          >
        </label>
        <div class="api-check-row">
          <button
            class="api-check-button"
            :disabled="checkingApi"
            @click="onCheckApiConnection"
          >
            {{ checkingApi ? '检测中...' : '检测链接' }}
          </button>
          <span
            v-if="apiStatus"
            class="api-status"
          >{{ apiStatus }}</span>
        </div>
        <label>
          反推模型
          <select
            v-model="reverseModel"
            class="api-model-select"
            @change="saveApiSettings"
          >
            <option
              v-for="model in availableReverseModels"
              :key="model"
              :value="model"
            >
              {{ model }}
            </option>
          </select>
        </label>
      </section>
      <hr>
      <button
        :disabled="importing"
        @click="onBatchImport"
      >
        {{ importing ? '导入中…' : '📂 批量导入提示词' }}
      </button>
      <button @click="onExport">
        导出 (.zip)
      </button>
      <button @click="onImport">
        导入 (.zip)
      </button>
      <section class="version-panel">
        <div class="version-header">
          <div>
            <strong>版本更新</strong>
            <p>检查 GitHub Release 是否有新版安装包。</p>
          </div>
          <button
            class="version-check-button"
            :disabled="checkingVersion"
            @click="onCheckUpdate"
          >
            {{ checkingVersion ? '检查中...' : '检查更新' }}
          </button>
        </div>
        <p
          v-if="updateResult"
          class="version-status"
        >
          当前版本 {{ updateResult.currentVersion }}，最新版本 {{ updateResult.latestVersion }}。
          <span v-if="updateResult.updateAvailable">发现新版本，可以下载更新。</span>
          <span v-else>已是最新版本。</span>
        </p>
        <p
          v-if="updateError"
          class="version-status error"
        >
          {{ updateError }}
        </p>
        <button
          v-if="updateResult?.updateAvailable"
          class="install-update-button"
          :disabled="installingUpdate"
          @click="onDownloadUpdate"
        >
          下载更新
        </button>
      </section>
      <div class="actions">
        <button @click="ui.closeSettings()">
          关闭
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}
.dialog {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  width: 390px;
  max-width: calc(100vw - 24px);
  max-height: calc(100vh - 32px);
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
input {
  width: 100%;
  min-width: 0;
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
}
select {
  width: 100%;
  min-width: 0;
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
}
.actions {
  display: flex;
  justify-content: flex-end;
}
.api-panel {
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 10px;
  background: #f8fafc;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.api-panel label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: #334155;
  font-size: 13px;
}
.api-header p,
.api-status {
  margin: 4px 0 0;
  color: #64748b;
  font-size: 12px;
  line-height: 1.4;
}
.api-check-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.version-panel {
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  padding: 10px;
  background: #f8fafc;
}
.version-header {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  justify-content: space-between;
}
.version-header p,
.version-status {
  margin: 4px 0 0;
  color: #64748b;
  font-size: 12px;
  line-height: 1.4;
}
.version-check-button,
.install-update-button {
  flex: 0 0 auto;
}
.install-update-button {
  margin-top: 8px;
}
.version-status.error {
  color: #b91c1c;
}
</style>

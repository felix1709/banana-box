<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { v4 as uuid } from 'uuid'
import { open } from '@tauri-apps/plugin-dialog'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
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

type SettingsTab = 'features' | 'api' | 'hotkeys' | 'import-export'

const settingsTabs: { id: SettingsTab; label: string }[] = [
  { id: 'features', label: '功能设置' },
  { id: 'api', label: 'API设置' },
  { id: 'hotkeys', label: '快捷键' },
  { id: 'import-export', label: '导入导出' },
]

const lib = useLibraryStore()
const ui = useUiStore()
const activeTab = ref<SettingsTab>('features')
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
const autostartEnabled = ref(false)
const loadingAutostart = ref(true)
const savingAutostart = ref(false)
const autostartError = ref('')

onMounted(async () => {
  await refreshAutostart()
})

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

async function refreshAutostart() {
  loadingAutostart.value = true
  autostartError.value = ''
  try {
    autostartEnabled.value = await isEnabled()
  } catch {
    autostartError.value = '读取开机启动状态失败'
  } finally {
    loadingAutostart.value = false
  }
}

async function onToggleAutostart(event: Event) {
  const checked = (event.target as HTMLInputElement).checked
  const previous = autostartEnabled.value
  autostartEnabled.value = checked
  savingAutostart.value = true
  autostartError.value = ''
  try {
    if (checked) {
      await enable()
    } else {
      await disable()
    }
    ui.showToast(checked ? '已开启开机启动' : '已关闭开机启动')
  } catch {
    autostartEnabled.value = previous
    autostartError.value = '保存开机启动设置失败'
  } finally {
    savingAutostart.value = false
  }
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

async function onBatchImport() {
  const dir = await open({ directory: true, multiple: false })
  if (!dir || Array.isArray(dir)) return
  importing.value = true
  ui.showToast('解析中...')
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
        // 忽略单张下载失败，继续导入文本内容。
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
    updateError.value = '下载更新失败，请稍后重试'
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
      <div class="dialog-header">
        <h3>设置</h3>
        <button
          class="close-button"
          aria-label="关闭设置"
          @click="ui.closeSettings()"
        >
          x
        </button>
      </div>

      <div class="settings-shell">
        <nav
          class="settings-tabs"
          aria-label="设置分类"
        >
          <button
            v-for="tab in settingsTabs"
            :key="tab.id"
            class="settings-tab"
            :class="{ active: activeTab === tab.id }"
            type="button"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </nav>

        <div class="settings-page">
          <section
            v-if="activeTab === 'features'"
            class="settings-section"
          >
            <div class="setting-row">
              <div>
                <strong>开机启动</strong>
                <p>随系统开启，自动启动工具。</p>
              </div>
              <label class="switch">
                <input
                  class="autostart-toggle"
                  type="checkbox"
                  :checked="autostartEnabled"
                  :disabled="loadingAutostart || savingAutostart"
                  @change="onToggleAutostart"
                >
                <span class="switch-track" />
              </label>
            </div>
            <p
              v-if="autostartError"
              class="status error"
            >
              {{ autostartError }}
            </p>

            <div class="version-panel">
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
            </div>
          </section>

          <section
            v-else-if="activeTab === 'api'"
            class="api-panel settings-section"
          >
            <div class="api-header">
              <strong>API 调用</strong>
              <p>用于反推图片提示词。</p>
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
                {{ checkingApi ? '检测中...' : '检测连接' }}
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

          <section
            v-else-if="activeTab === 'hotkeys'"
            class="settings-section"
          >
            <label class="field">
              全局快捷键
              <input v-model="hotkey">
            </label>
            <button @click="saveHotkey">
              保存快捷键
            </button>
          </section>

          <section
            v-else
            class="settings-section"
          >
            <button
              :disabled="importing"
              @click="onBatchImport"
            >
              {{ importing ? '导入中...' : '批量导入提示词' }}
            </button>
            <button @click="onExport">
              导出 (.zip)
            </button>
            <button @click="onImport">
              导入 (.zip)
            </button>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.68);
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(5px);
  z-index: 20;
}
.dialog {
  background: var(--bb-surface);
  padding: 16px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-lg);
  width: 560px;
  max-width: calc(100vw - 24px);
  max-height: calc(100vh - 32px);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-shadow: var(--bb-shadow-dialog);
}
.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.dialog h3 {
  margin: 0 0 2px;
  color: var(--bb-text);
  font-size: 16px;
}
.close-button {
  width: 32px;
  height: 32px;
  padding: 0;
  flex: 0 0 auto;
  line-height: 1;
}
.settings-shell {
  min-height: 0;
  display: grid;
  grid-template-columns: 132px minmax(0, 1fr);
  gap: 14px;
}
.settings-tabs {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.settings-tab {
  width: 100%;
  min-height: 34px;
  padding: 0 10px;
  text-align: left;
  background: transparent;
  color: var(--bb-text-muted);
  box-shadow: none;
}
.settings-tab:hover {
  color: var(--bb-text);
  background: var(--bb-primary-soft);
}
.settings-tab.active {
  border-color: var(--bb-border-strong);
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
  font-weight: 700;
}
.settings-page {
  min-width: 0;
  max-height: min(520px, calc(100vh - 116px));
  overflow-y: auto;
  overflow-x: hidden;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
  padding-right: 4px;
}
.settings-section,
.api-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
input,
select {
  width: 100%;
  min-width: 0;
  padding: 7px 8px;
}
.field,
.api-panel label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--bb-text);
  font-size: 13px;
}
.setting-row,
.version-panel,
.api-panel {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  padding: 11px;
  background: var(--bb-surface-soft);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
}
.setting-row p,
.api-header p,
.api-status,
.version-header p,
.version-status,
.status {
  margin: 4px 0 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  line-height: 1.4;
  overflow-wrap: anywhere;
}
.switch {
  flex: 0 0 auto;
  position: relative;
  display: inline-flex;
  width: 44px;
  height: 24px;
}
.switch input {
  position: absolute;
  opacity: 0;
  width: 1px;
  height: 1px;
}
.switch-track {
  width: 44px;
  height: 24px;
  border-radius: 999px;
  background: var(--bb-surface-muted);
  border: 1px solid var(--bb-border);
  transition:
    background 0.16s ease,
    border-color 0.16s ease;
}
.switch-track::after {
  content: '';
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: 999px;
  background: var(--bb-text-muted);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.34);
  transition:
    transform 0.16s ease,
    background 0.16s ease;
}
.switch input:checked + .switch-track {
  background: var(--bb-primary-soft);
  border-color: var(--bb-border-strong);
}
.switch input:checked + .switch-track::after {
  background: var(--bb-primary);
  transform: translateX(20px);
}
.switch input:focus-visible + .switch-track {
  outline: none;
  box-shadow: var(--bb-focus);
}
.api-check-row,
.version-header {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  justify-content: space-between;
}
.api-check-button,
.version-check-button,
.install-update-button {
  flex: 0 0 auto;
}
.install-update-button {
  margin-top: 8px;
}
.error,
.version-status.error {
  color: var(--bb-danger);
}

@media (max-width: 560px) {
  .dialog {
    width: calc(100vw - 24px);
  }
  .settings-shell {
    grid-template-columns: 1fr;
  }
  .settings-tabs {
    flex-direction: row;
    overflow-x: auto;
    padding-bottom: 2px;
  }
  .settings-tab {
    flex: 0 0 auto;
    width: auto;
    text-align: center;
    white-space: nowrap;
  }
  .settings-page {
    max-height: calc(100vh - 170px);
  }
}
</style>

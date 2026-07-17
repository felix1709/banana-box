<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { v4 as uuid } from 'uuid'
import { open } from '@tauri-apps/plugin-dialog'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { useLibraryStore } from '@/stores/library'
import { useProviderStore } from '@/stores/providers'
import { useUiStore } from '@/stores/ui'
import { useAuthStore } from '@/stores/auth'
import { useCloudSessionStore } from '@/stores/cloudSession'
import {
  copyToClipboard,
  exportLibrary,
  readImportDir,
  downloadImage,
} from '@/lib/ipc'
import authWorkspaceSql from '../../supabase/migrations/0001_auth_workspaces.sql?raw'
import contentCollaborationSql from '../../supabase/migrations/0002_content_collaboration_schema.sql?raw'
import inviteAcceptanceRealtimeSql from '../../supabase/migrations/0003_invite_acceptance_realtime.sql?raw'
import workspaceBootstrapFixSql from '../../supabase/migrations/0004_workspace_bootstrap_fix.sql?raw'
import profileBootstrapFixSql from '../../supabase/migrations/0005_profile_bootstrap_fix.sql?raw'
import dailyTaskRemindersSql from '../../supabase/migrations/0006_daily_task_reminders.sql?raw'
import projectLevelCollaborationSql from '../../supabase/migrations/0007_project_level_collaboration.sql?raw'
import projectInviteNotificationsSql from '../../supabase/migrations/0008_project_invite_notifications.sql?raw'
import profileDisplayNameRpcSql from '../../supabase/migrations/0009_profile_display_name_rpc.sql?raw'
import notificationsInsertPolicySql from '../../supabase/migrations/0010_notifications_insert_policy.sql?raw'
import inviteDigestExtensionPathSql from '../../supabase/migrations/0011_invite_digest_extension_path.sql?raw'
import inviteAcceptanceConflictTargetsSql from '../../supabase/migrations/0012_invite_acceptance_conflict_targets.sql?raw'
import projectScheduleChangeRequestsSql from '../../supabase/migrations/0013_project_schedule_change_requests.sql?raw'
import projectCollaborationRlsFixesSql from '../../supabase/migrations/0014_project_collaboration_rls_fixes.sql?raw'
import sharedPromptLibrarySql from '../../supabase/migrations/0015_shared_prompt_library.sql?raw'
import sharedPromptAdminDeleteSql from '../../supabase/migrations/0016_shared_prompt_admin_delete.sql?raw'
import { checkAiProviderConnection } from '@/lib/provider-ipc'
import {
  commitLegacyImport,
  discardLegacyImportPreview,
  inspectLegacyImport,
} from '@/lib/backup-ipc'
import { checkAppUpdate, installAppUpdate } from '@/lib/updater'
import { parseFile } from '@/lib/parse'
import type { AiProvider, CheckAiProviderConnectionResult, Prompt, ProviderKind } from '@/types'
import type { AppUpdateResult } from '@/lib/updater'
import type { LegacyImportPreview } from '@/lib/backup-ipc'

type SettingsTab = 'features' | 'api' | 'hotkeys' | 'import-export'

const settingsTabs: { id: SettingsTab; label: string }[] = [
  { id: 'features', label: '功能设置' },
  { id: 'api', label: 'API设置' },
  { id: 'hotkeys', label: '快捷键' },
  { id: 'import-export', label: '导入导出' },
]

const lib = useLibraryStore()
const providers = useProviderStore()
const ui = useUiStore()
const auth = useAuthStore()
const cloud = useCloudSessionStore()
const activeTab = ref<SettingsTab>('features')
const hotkey = ref(lib.library.settings.hotkey)
const cloudSupabaseUrl = ref('')
const cloudAnonKey = ref('')
const cloudEnabled = ref(false)
const cloudStatus = ref('')
const apiBaseUrl = ref('')
const apiKey = ref('')
const apiProviderId = ref<ProviderKind>('reverse-image')
const apiModel = ref('')
const availableModels = ref<string[]>([])
const apiTemperature = ref(0.7)
const apiContextWindowTokens = ref(16000)
const checkingApi = ref(false)
const apiStatus = ref('')
const importing = ref(false)
const inspectingLegacyImport = ref(false)
const committingLegacyImport = ref(false)
const legacyImportPreview = ref<LegacyImportPreview | null>(null)
const overwriteLegacyCredential = ref(false)
const legacyImportError = ref('')
const checkingVersion = ref(false)
const installingUpdate = ref(false)
const updateResult = ref<AppUpdateResult | null>(null)
const updateError = ref('')
const autostartEnabled = ref(false)
const loadingAutostart = ref(true)
const savingAutostart = ref(false)
const autostartError = ref('')
const cloudSetupSql = `${authWorkspaceSql.trim()}\n\n${contentCollaborationSql.trim()}\n\n${inviteAcceptanceRealtimeSql.trim()}\n\n${workspaceBootstrapFixSql.trim()}\n\n${profileBootstrapFixSql.trim()}\n\n${dailyTaskRemindersSql.trim()}\n\n${projectLevelCollaborationSql.trim()}\n\n${projectInviteNotificationsSql.trim()}\n\n${profileDisplayNameRpcSql.trim()}\n\n${notificationsInsertPolicySql.trim()}\n\n${inviteDigestExtensionPathSql.trim()}\n\n${inviteAcceptanceConflictTargetsSql.trim()}\n\n${projectScheduleChangeRequestsSql.trim()}\n\n${projectCollaborationRlsFixesSql.trim()}\n\n${sharedPromptLibrarySql.trim()}\n\n${sharedPromptAdminDeleteSql.trim()}\n`

onMounted(async () => {
  await Promise.all([refreshAutostart(), loadApiProviders(), loadCloudSettings()])
})

function saveHotkey() {
  lib.library.settings.hotkey = hotkey.value
  lib.persist()
  ui.showToast('已保存')
}

function pickPreferredModel(models: string[]) {
  if (models.includes(apiModel.value)) return apiModel.value
  if (apiProviderId.value === 'storyboard' && models.includes('glm-5.2')) return 'glm-5.2'
  return models[0] ?? ''
}

function cloudStatusText() {
  if (cloudStatus.value) return cloudStatus.value
  if (cloud.readiness === 'configured') return '云端配置已保存'
  if (cloud.readiness === 'invalid') return '云端配置无效'
  return '本地离线模式'
}

function canManageCloudSettings() {
  return !auth.user || auth.isCloudAdmin
}

async function loadCloudSettings() {
  await cloud.load()
  cloudSupabaseUrl.value = cloud.config?.supabaseUrl ?? ''
  cloudEnabled.value = cloud.config?.cloudEnabled ?? false
  cloudAnonKey.value = ''
}

async function saveCloudSettings() {
  await cloud.save({
    supabaseUrl: cloudSupabaseUrl.value,
    anonKey: cloudAnonKey.value,
    cloudEnabled: cloudEnabled.value,
  })
  cloudAnonKey.value = ''
  cloudStatus.value = cloud.error ? `保存失败：${cloud.error}` : '云端配置已保存'
}

async function copyCloudSetupSql() {
  await copyToClipboard(cloudSetupSql)
  cloudStatus.value = 'Supabase 建表 SQL 已复制'
}

function trimTrailingSlashes(value: string) {
  return value.trim().replace(/\/+$/, '')
}

function withDefaultApiVersion(baseUrl: string) {
  const normalized = trimTrailingSlashes(baseUrl)
  try {
    const parsed = new URL(normalized)
    if (parsed.pathname === '' || parsed.pathname === '/') {
      parsed.pathname = '/v1'
      return trimTrailingSlashes(parsed.toString())
    }
  } catch {
    return normalized
  }
  return normalized
}

function providerEndpointsFromBaseUrl(baseUrl: string) {
  const normalized = withDefaultApiVersion(baseUrl)
  return {
    baseUrl: normalized,
    modelsUrl: `${normalized}/models`,
    chatCompletionsUrl: `${normalized}/chat/completions`,
  }
}

function apiBaseUrlFromProvider(provider: AiProvider) {
  const modelsUrl = trimTrailingSlashes(provider.modelsUrl)
  const chatUrl = trimTrailingSlashes(provider.chatCompletionsUrl)
  if (
    modelsUrl.endsWith('/models')
    && chatUrl.endsWith('/chat/completions')
  ) {
    const modelBaseUrl = modelsUrl.slice(0, -'/models'.length)
    const chatBaseUrl = chatUrl.slice(0, -'/chat/completions'.length)
    if (modelBaseUrl === chatBaseUrl) return withDefaultApiVersion(modelBaseUrl)
  }
  return withDefaultApiVersion(provider.baseUrl)
}

function hasUnsavedApiConnectionInput() {
  const provider = providers.byId(apiProviderId.value)
  const endpoints = providerEndpointsFromBaseUrl(apiBaseUrl.value)
  return (
    Boolean(apiKey.value.trim())
    || endpoints.modelsUrl !== trimTrailingSlashes(provider?.modelsUrl ?? '')
    || endpoints.chatCompletionsUrl !== trimTrailingSlashes(provider?.chatCompletionsUrl ?? '')
  )
}

function connectionStatusMessage(result: CheckAiProviderConnectionResult) {
  if (result.ok) return '连接成功'

  return (
    {
      PROVIDER_CREDENTIALS_REQUIRED: '请先保存 API Key',
      INVALID_PROVIDER_URL: 'Provider 地址无效',
      PROVIDER_TIMEOUT: '连接超时，请稍后重试',
      PROVIDER_HTTP_ERROR: '服务返回异常，请检查配置',
    }[result.message] ?? '连接失败，请检查 Provider 设置'
  )
}

function applyProvider(provider: AiProvider) {
  apiProviderId.value = provider.kind
  apiBaseUrl.value = apiBaseUrlFromProvider(provider)
  const selectedModel = provider.defaultModel ?? provider.probedModel ?? ''
  const nextModels = provider.availableModels.length
    ? provider.availableModels
    : availableModels.value
  availableModels.value = selectedModel && !nextModels.includes(selectedModel)
    ? [selectedModel, ...nextModels]
    : [...nextModels]
  apiModel.value = selectedModel || pickPreferredModel(availableModels.value)
  apiTemperature.value = provider.temperature ?? 0.7
  apiContextWindowTokens.value = provider.contextWindowTokens ?? 16000
}

async function loadApiProviders() {
  try {
    await Promise.all([providers.load('reverse-image'), providers.load('storyboard')])
    const provider = providers.byId('reverse-image')
    if (provider) applyProvider(provider)
  } catch {
    apiStatus.value = '读取 API 设置失败'
  }
}

function selectApiProvider() {
  apiKey.value = ''
  apiStatus.value = ''
  const provider = providers.byId(apiProviderId.value)
  if (provider) applyProvider(provider)
}

async function saveApiSettings() {
  try {
    const existing = providers.byId(apiProviderId.value)
    if (!existing) {
      apiStatus.value = '未找到 API 服务'
      return
    }

    const endpoints = providerEndpointsFromBaseUrl(apiBaseUrl.value)
    const saved = await providers.save({
      provider: {
        id: existing.id,
        kind: existing.kind,
        displayName: existing.displayName,
        baseUrl: endpoints.baseUrl,
        modelsUrl: endpoints.modelsUrl,
        chatCompletionsUrl: endpoints.chatCompletionsUrl,
        defaultModel: apiModel.value || null,
        temperature: existing.kind === 'storyboard' ? Number(apiTemperature.value) : undefined,
        contextWindowTokens: existing.kind === 'storyboard' ? Number(apiContextWindowTokens.value) : undefined,
        confirmCrossOrigin: false,
      },
      apiKey: apiKey.value,
    })
    applyProvider(saved)
    apiStatus.value = '已保存'
  } catch {
    apiStatus.value = '保存失败，请检查地址和权限'
  } finally {
    apiKey.value = ''
  }
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
    if (hasUnsavedApiConnectionInput()) {
      apiStatus.value = '请先保存 API 设置后再检测连接'
      return
    }
    const result = await checkAiProviderConnection(apiProviderId.value)
    if (result.models.length) {
      availableModels.value = result.models
      apiModel.value = pickPreferredModel(result.models)
    }
    apiStatus.value = connectionStatusMessage(result)
  } catch {
    apiStatus.value = '连接失败，请检查 Provider 设置'
  } finally {
    checkingApi.value = false
  }
}

async function onExport() {
  await exportLibrary()
  ui.showToast('已导出')
}

async function discardLegacyImport() {
  const token = legacyImportPreview.value?.token
  legacyImportPreview.value = null
  overwriteLegacyCredential.value = false
  if (!token) return
  try {
    await discardLegacyImportPreview(token)
  } catch {
    // The backend also removes stale previews on the next app startup.
  }
}

async function onInspectLegacyImport() {
  const picked = await open({
    filters: [{ name: 'legacy prompt library', extensions: ['json', 'zip'] }],
    multiple: false,
  })
  if (!picked || Array.isArray(picked)) return

  await discardLegacyImport()
  inspectingLegacyImport.value = true
  legacyImportError.value = ''
  try {
    legacyImportPreview.value = await inspectLegacyImport(picked)
  } catch {
    legacyImportError.value = '无法检查旧版提示词库'
  } finally {
    inspectingLegacyImport.value = false
  }
}

async function onCommitLegacyImport() {
  const preview = legacyImportPreview.value
  if (!preview) return
  if (preview.credentialConflict && !overwriteLegacyCredential.value) {
    legacyImportError.value = '请确认是否覆盖已有 API Key'
    return
  }

  committingLegacyImport.value = true
  legacyImportError.value = ''
  try {
    const committed = await commitLegacyImport(preview.token, overwriteLegacyCredential.value)
    lib.hydrate(committed.library)
    try {
      await loadApiProviders()
    } catch {
      // The library import itself has already committed at this point.
    }
    legacyImportPreview.value = null
    overwriteLegacyCredential.value = false
    ui.showToast(`已导入 ${committed.promptsImported} 条提示词`)
  } catch {
    legacyImportError.value = '导入失败，原有数据未被覆盖'
  } finally {
    committingLegacyImport.value = false
  }
}

async function closeSettings() {
  await discardLegacyImport()
  ui.closeSettings()
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
    @click.self="closeSettings"
  >
    <div class="dialog scrollable-dialog">
      <div class="dialog-header">
        <h3>设置</h3>
        <button
          class="close-button"
          aria-label="关闭设置"
          @click="closeSettings"
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

            <section
              v-if="canManageCloudSettings()"
              class="cloud-config-section"
            >
              <div class="section-heading">
                <strong>云端协作</strong>
                <p>配置 Supabase 后，下一阶段可登录并启用用户云空间。</p>
              </div>
              <label>
                Supabase URL
                <input
                  v-model="cloudSupabaseUrl"
                  class="cloud-url-input"
                  placeholder="https://example.supabase.co"
                >
              </label>
              <label>
                Supabase anon key
                <input
                  v-model="cloudAnonKey"
                  class="cloud-anon-key-input"
                  type="password"
                  placeholder="留空表示不修改"
                >
              </label>
              <label class="cloud-enabled-row">
                <input
                  v-model="cloudEnabled"
                  class="cloud-enabled-toggle"
                  type="checkbox"
                >
                启用云端配置
              </label>
              <div class="cloud-config-actions">
                <button
                  class="cloud-save-button"
                  type="button"
                  :disabled="cloud.saving"
                  @click="saveCloudSettings"
                >
                  {{ cloud.saving ? '保存中...' : '保存云端配置' }}
                </button>
                <button
                  class="cloud-sql-copy-button"
                  type="button"
                  @click="copyCloudSetupSql"
                >
                  复制建表 SQL
                </button>
                <span class="cloud-status">{{ cloudStatusText() }}</span>
              </div>
            </section>
          </section>

          <section
            v-else-if="activeTab === 'api'"
            class="api-panel settings-section"
          >
            <div class="api-header">
              <strong>API 调用</strong>
              <p>{{ apiProviderId === 'storyboard' ? '用于故事板 Agent 对话。' : '用于反推图片提示词。' }}</p>
            </div>
            <label>
              服务
              <select
                v-model="apiProviderId"
                data-field="api-provider"
                @change="selectApiProvider"
              >
                <option value="reverse-image">反推图片</option>
                <option value="storyboard">故事板 Agent</option>
              </select>
            </label>
            <label>
              API URL
              <input
                v-model="apiBaseUrl"
                class="api-base-url-input"
                placeholder="https://api.example.com/v1"
              >
            </label>
            <label>
              API Key
              <input
                v-model="apiKey"
                class="api-key-input"
                type="password"
                placeholder="留空表示不修改"
              >
            </label>
            <template v-if="apiProviderId === 'storyboard'">
              <label>
                温度
                <input
                  v-model.number="apiTemperature"
                  data-field="api-temperature"
                  max="2"
                  min="0"
                  step="0.1"
                  type="number"
                >
              </label>
              <label>
                上下文长度
                <input
                  v-model.number="apiContextWindowTokens"
                  data-field="api-context-window"
                  max="128000"
                  min="512"
                  step="1"
                  type="number"
                >
              </label>
            </template>
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
              {{ apiProviderId === 'storyboard' ? '故事板模型' : '反推模型' }}
              <select
                v-model="apiModel"
                class="api-model-select"
                @change="saveApiSettings"
              >
                <option
                  v-for="model in availableModels"
                  :key="model"
                  :value="model"
                >
                  {{ model }}
                </option>
              </select>
            </label>
            <button
              class="api-save-button"
              type="button"
              @click="saveApiSettings"
            >
              保存 API 设置
            </button>
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
            <button
              class="legacy-import-button"
              data-action="inspect-legacy-import"
              :disabled="inspectingLegacyImport || committingLegacyImport"
              @click="onInspectLegacyImport"
            >
              {{ inspectingLegacyImport ? '检查中...' : '导入旧版提示词库' }}
            </button>
            <div
              v-if="legacyImportPreview || legacyImportError"
              class="legacy-import-preview"
            >
              <p v-if="legacyImportPreview">
                提示词 {{ legacyImportPreview.promptCount }} 条，分类 {{ legacyImportPreview.categoryCount }} 个
              </p>
              <label
                v-if="legacyImportPreview?.credentialConflict"
                class="legacy-overwrite"
              >
                <input
                  v-model="overwriteLegacyCredential"
                  type="checkbox"
                >
                覆盖已有 API Key
              </label>
              <p
                v-if="legacyImportError"
                class="status error"
              >
                {{ legacyImportError }}
              </p>
              <div
                v-if="legacyImportPreview"
                class="legacy-import-actions"
              >
                <button
                  type="button"
                  data-action="discard-legacy-import"
                  :disabled="committingLegacyImport"
                  @click="discardLegacyImport"
                >
                  取消
                </button>
                <button
                  type="button"
                  data-action="commit-legacy-import"
                  :disabled="committingLegacyImport"
                  @click="onCommitLegacyImport"
                >
                  {{ committingLegacyImport ? '导入中...' : '确认导入' }}
                </button>
              </div>
            </div>
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
  z-index: 260;
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
.cloud-config-section,
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
.section-heading p,
.cloud-status,
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
.cloud-config-actions,
.version-header {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  justify-content: space-between;
}
.cloud-config-section {
  display: grid;
  gap: 10px;
}
.cloud-config-section label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--bb-text);
  font-size: 13px;
}
.cloud-enabled-row {
  flex-direction: row !important;
  align-items: center;
  gap: 8px;
}
.cloud-enabled-row input {
  width: auto;
  min-height: 0;
}
.cloud-config-actions {
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-start;
}
.legacy-import-preview {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 9px;
  border: 1px solid var(--bb-border);
  background: var(--bb-surface-soft);
}
.legacy-import-preview p {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 12px;
}
.legacy-overwrite,
.legacy-import-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.legacy-overwrite {
  color: var(--bb-text);
  font-size: 12px;
}
.legacy-overwrite input {
  width: auto;
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

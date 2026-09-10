<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Activity, ExternalLink, Play, RefreshCw, Square, Trash2 } from '@lucide/vue'
import MediaToolProgressDialog from '@/components/MediaToolProgressDialog.vue'
import {
  cleanPiWebRuntime,
  getPiWebConfigStatus,
  getPiWebChatHealth,
  getPiWebRuntimeStatus,
  getPiWebStatus,
  installPiWebRuntime,
  openPiWeb,
  repairPiWebConfig,
  repairPiWebModelCompatibility,
  setPiWebRuntimeDirectory,
  startPiWeb,
  stopPiWeb,
  type PiWebChatHealth,
  type PiWebConfigRepairResult,
  type PiWebConfigStatus,
  type PiWebProgressPayload,
  type PiWebRepairResult,
  type PiWebRuntimeStatus,
  type PiWebStatus,
} from '@/lib/piWebIpc'

const DEFAULT_PI_WEB_BASE_URL = 'https://ai.leihuo.netease.com/v1'
const DEFAULT_PI_WEB_PROVIDER = 'leihuo'
const DEFAULT_PI_WEB_MODEL = 'deepseek-v4-flash'

const status = ref<PiWebStatus | null>(null)
const chatHealth = ref<PiWebChatHealth | null>(null)
const repairResult = ref<PiWebRepairResult | null>(null)
const configStatus = ref<PiWebConfigStatus | null>(null)
const configRepairResult = ref<PiWebConfigRepairResult | null>(null)
const configBaseUrl = ref(DEFAULT_PI_WEB_BASE_URL)
const configApiKey = ref('')
const configError = ref('')
const busy = ref(false)
const healthBusy = ref(false)
const repairBusy = ref(false)
const configBusy = ref(false)
const configRepairBusy = ref(false)
const runtime = ref<PiWebRuntimeStatus | null>(null)
const runtimeBusy = ref(false)
const runtimeMessage = ref('')
const runtimeError = ref('')
const installDialogOpen = ref(false)
const installStatus = ref<'idle' | 'running' | 'success' | 'error'>('idle')
const installProgress = ref(0)
const installMessageText = ref('')
const installError = ref('')
const installLogs = ref<string[]>([])
const installOperationId = ref('')
let unlistenInstall: UnlistenFn | null = null

const stateLabel = computed(() => status.value?.state ?? 'checking')
const stateText = computed(() => {
  switch (stateLabel.value) {
    case 'missingRuntime':
      return '缺少运行环境'
    case 'stopped':
      return '未启动'
    case 'checking':
      return '检查中'
    case 'starting':
      return '启动中'
    case 'running':
      return '运行中'
    case 'error':
      return '异常'
    default:
      return '未知'
  }
})
const canCheckChat = computed(() => Boolean(status.value?.canOpen))
const canRepairDeveloperRole = computed(() => {
  const detail = chatHealth.value?.detail ?? ''
  return chatHealth.value?.state === 'error' && detail.includes('developer is not one of')
})
const normalizedBaseUrl = computed(() => configBaseUrl.value.trim().replace(/\/+$/, ''))
const configModelLabel = computed(() => {
  const provider = configStatus.value?.defaultProvider || DEFAULT_PI_WEB_PROVIDER
  const model = configStatus.value?.defaultModel || DEFAULT_PI_WEB_MODEL
  return `${provider} / ${model}`
})
const canRepairConfig = computed(() => {
  return Boolean(normalizedBaseUrl.value && configApiKey.value.trim() && !configRepairBusy.value)
})
const configProgressText = computed(() => {
  if (configRepairBusy.value) return '正在写入 PI-WEB 配置，请稍候…'
  if (configBusy.value) return '正在检查 PI-WEB 配置状态…'
  if (configError.value) return configError.value
  if (configRepairResult.value) return `${configRepairResult.value.message} ${configRepairResult.value.detail}`
  if (configStatus.value?.needsRepair) return '检测到配置不完整，请填写 URL 和 API Key 后点击一键配置。'
  if (configStatus.value) return configStatus.value.message
  return '填写 URL 和 API Key 后即可一键写入 PI-WEB 配置。'
})
const primaryAction = computed(() => {
  if (!status.value) return 'refresh'
  if (status.value.canOpen) return 'open'
  if (status.value.canStart) return 'start'
  return 'refresh'
})
const primaryLabel = computed(() => {
  if (busy.value) return '处理中'
  if (primaryAction.value === 'open') return '打开 PI-Web'
  if (primaryAction.value === 'start') return '启动 PI-Web'
  return '重新检查'
})

const runtimeStateText = computed(() => {
  switch (runtime.value?.state) {
    case 'notInstalled':
      return '未安装'
    case 'updateAvailable':
      return '版本可更新'
    case 'ready':
      return '已就绪'
    case 'error':
      return '异常'
    default:
      return '检查中'
  }
})
const runtimeSourceText = computed(() => {
  switch (runtime.value?.source) {
    case 'managed':
      return '一键下载'
    case 'custom':
      return '手动指定'
    case 'bundled':
      return '安装包内置'
    default:
      return '未安装'
  }
})
const runtimeNodeText = computed(() => (runtime.value?.nodePath ? 'Node 已就绪' : 'Node 缺失'))
const runtimeVersionText = computed(() =>
  runtime.value?.version ? `版本 ${runtime.value.version}` : '未安装',
)
const runtimeLatestText = computed(() =>
  runtime.value?.latestVersion ? `最新 ${runtime.value.latestVersion}` : '最新版本未知',
)
const runtimeInstallLabel = computed(() => (runtime.value?.canInstall ? '一键下载配置' : '更新 PI-WEB'))

function describeError(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}

async function refreshRuntime() {
  if (runtimeBusy.value || installStatus.value === 'running') return
  runtimeBusy.value = true
  runtimeError.value = ''
  try {
    runtime.value = await getPiWebRuntimeStatus()
  } catch (error) {
    runtimeError.value = describeError(error)
  } finally {
    runtimeBusy.value = false
  }
}

function appendInstallLog(line: string) {
  installLogs.value = [...installLogs.value, line].slice(-80)
}

async function startInstallListener() {
  unlistenInstall?.()
  unlistenInstall = await listen<PiWebProgressPayload>('pi-web-progress', (event) => {
    const payload = event.payload
    if (payload.operationId !== installOperationId.value) return
    installProgress.value = Math.max(installProgress.value, payload.progress)
    installMessageText.value = payload.message
    appendInstallLog(payload.detail ? `${payload.message}：${payload.detail}` : payload.message)
    if (payload.level === 'error') installStatus.value = 'error'
    if (payload.level === 'success') installStatus.value = 'success'
  })
}

async function runRuntimeInstall() {
  if (installStatus.value === 'running') return
  installOperationId.value = `pi-web-install-${Date.now()}`
  installDialogOpen.value = true
  installStatus.value = 'running'
  installProgress.value = 0
  installMessageText.value = '正在检查运行环境'
  installError.value = ''
  installLogs.value = []
  runtimeMessage.value = ''
  runtimeError.value = ''

  if (status.value?.canStop) {
    try {
      status.value = await stopPiWeb()
    } catch {
      // 停止失败不阻塞安装；真正切换版本时如果需要会再报错。
    }
  }

  await startInstallListener()
  try {
    runtime.value = await installPiWebRuntime(installOperationId.value)
    installStatus.value = 'success'
    installProgress.value = 100
    installMessageText.value = runtime.value.message
    runtimeMessage.value = 'PI-WEB 已就绪，点击「启动 PI-Web」即可使用。'
    await refresh()
  } catch (error) {
    installStatus.value = 'error'
    installError.value = describeError(error)
  }
}

function closeInstallDialog() {
  if (installStatus.value === 'running') return
  installDialogOpen.value = false
}

async function pickRuntimeDirectory() {
  if (runtimeBusy.value) return
  const picked = await open({ directory: true, multiple: false })
  if (!picked || Array.isArray(picked)) return
  runtimeBusy.value = true
  runtimeError.value = ''
  try {
    runtime.value = await setPiWebRuntimeDirectory(picked)
    runtimeMessage.value = '已切换到手动指定的 PI-WEB 目录。'
  } catch (error) {
    runtimeError.value = describeError(error)
  } finally {
    runtimeBusy.value = false
  }
}

async function resetRuntimeDirectory() {
  if (runtimeBusy.value) return
  runtimeBusy.value = true
  runtimeError.value = ''
  try {
    runtime.value = await setPiWebRuntimeDirectory(null)
    runtimeMessage.value = '已取消手动指定的目录。'
  } catch (error) {
    runtimeError.value = describeError(error)
  } finally {
    runtimeBusy.value = false
  }
}

async function cleanRuntime() {
  if (runtimeBusy.value) return
  const confirmed =
    typeof window.confirm !== 'function' ||
    window.confirm(
      '只会删除 Banana Box 一键下载的 PI-WEB；安装包内置的旧版和你自己的配置都不会动。确定继续吗？',
    )
  if (!confirmed) return
  runtimeBusy.value = true
  runtimeError.value = ''
  try {
    runtime.value = await cleanPiWebRuntime()
    runtimeMessage.value = '已清理一键下载的 PI-WEB。'
    await refresh()
  } catch (error) {
    runtimeError.value = describeError(error)
  } finally {
    runtimeBusy.value = false
  }
}

async function refresh() {
  busy.value = true
  try {
    status.value = await getPiWebStatus()
  } finally {
    busy.value = false
  }
}

async function runPrimaryAction() {
  if (busy.value) return
  busy.value = true
  try {
    if (primaryAction.value === 'open') status.value = await openPiWeb()
    else if (primaryAction.value === 'start') status.value = await startPiWeb()
    else status.value = await getPiWebStatus()
  } finally {
    busy.value = false
  }
}

async function refreshConfigStatus() {
  if (configBusy.value) return
  configBusy.value = true
  configError.value = ''
  try {
    configStatus.value = await getPiWebConfigStatus()
  } catch (error) {
    configError.value = error instanceof Error ? error.message : String(error)
  } finally {
    configBusy.value = false
  }
}

async function repairConfig() {
  if (!canRepairConfig.value) return
  configRepairBusy.value = true
  configError.value = ''
  configRepairResult.value = null
  try {
    configRepairResult.value = await repairPiWebConfig(configApiKey.value, normalizedBaseUrl.value)
    configStatus.value = configRepairResult.value.status
  } catch (error) {
    configError.value = error instanceof Error ? error.message : String(error)
  } finally {
    configApiKey.value = ''
    configRepairBusy.value = false
  }
}

async function checkChatHealth() {
  if (healthBusy.value || !canCheckChat.value) return
  healthBusy.value = true
  try {
    chatHealth.value = await getPiWebChatHealth()
    repairResult.value = null
  } finally {
    healthBusy.value = false
  }
}

async function repairModelCompatibility() {
  if (repairBusy.value || !canRepairDeveloperRole.value) return
  repairBusy.value = true
  try {
    repairResult.value = await repairPiWebModelCompatibility()
    status.value = await getPiWebStatus()
  } finally {
    repairBusy.value = false
  }
}

async function stop() {
  if (busy.value || !status.value?.canStop) return
  busy.value = true
  try {
    status.value = await stopPiWeb()
    chatHealth.value = null
  } finally {
    busy.value = false
  }
}

onMounted(() => {
  refresh()
  void refreshConfigStatus()
  void refreshRuntime()
})

onBeforeUnmount(() => {
  unlistenInstall?.()
})
</script>

<template>
  <main class="pi-web-page">
    <header class="pi-web-header">
      <div>
        <p>本地智能体控制台</p>
        <h2>PI-Web</h2>
      </div>
      <span
        class="pi-web-state"
        :data-state="stateLabel"
      >
        {{ stateText }}
      </span>
    </header>

    <section class="pi-web-hero">
      <div class="pi-web-copy">
        <strong>{{ status?.message || '正在检查 PI-Web 状态' }}</strong>
        <span>{{ status?.url || 'http://127.0.0.1:30141' }}</span>
      </div>
      <div class="pi-web-actions">
        <button
          class="pi-web-primary"
          data-action="start-pi-web"
          type="button"
          :disabled="busy || (!status?.canStart && !status?.canOpen && primaryAction !== 'refresh')"
          @click="runPrimaryAction"
        >
          <ExternalLink
            v-if="primaryAction === 'open'"
            :size="16"
          />
          <Play
            v-else-if="primaryAction === 'start'"
            :size="16"
          />
          <RefreshCw
            v-else
            :size="16"
          />
          {{ primaryLabel }}
        </button>
        <button
          data-action="check-pi-web-chat"
          type="button"
          :disabled="healthBusy || !canCheckChat"
          @click="checkChatHealth"
        >
          <Activity :size="15" />
          {{ healthBusy ? '检测中' : '检测对话' }}
        </button>
        <button
          data-action="stop-pi-web"
          type="button"
          :disabled="busy || !status?.canStop"
          @click="stop"
        >
          <Square :size="15" />
          停止
        </button>
      </div>
    </section>

    <section
      class="pi-web-diagnostics pi-web-runtime-card"
      aria-live="polite"
    >
      <div class="pi-web-card-heading">
        <div>
          <h3>PI-WEB 运行环境</h3>
          <p class="pi-web-health-title">
            {{ runtime?.message || '正在检查本地 PI-WEB…' }}
          </p>
        </div>
        <button
          data-action="refresh-pi-web-runtime"
          type="button"
          :disabled="runtimeBusy || installStatus === 'running'"
          @click="refreshRuntime"
        >
          <RefreshCw :size="14" />
          {{ runtimeBusy ? '检查中' : '重新检测' }}
        </button>
      </div>

      <div class="pi-web-runtime-grid">
        <span :data-ready="Boolean(runtime?.installed)">{{ runtimeStateText }}</span>
        <span :data-ready="runtime?.source === 'managed'">来源：{{ runtimeSourceText }}</span>
        <span :data-ready="Boolean(runtime?.version)">{{ runtimeVersionText }}</span>
        <span :data-ready="Boolean(runtime?.latestVersion)">{{ runtimeLatestText }}</span>
        <span :data-ready="Boolean(runtime?.nodePath)">{{ runtimeNodeText }}</span>
      </div>

      <p
        v-if="runtime?.installDir"
        class="pi-web-health-title"
      >
        目录：{{ runtime.installDir }}
      </p>
      <p v-if="runtime?.detail">
        {{ runtime.detail }}
      </p>

      <div class="pi-web-card-actions">
        <button
          v-if="runtime?.canInstall || runtime?.canUpdate"
          data-action="install-pi-web-runtime"
          type="button"
          :disabled="runtimeBusy || installStatus === 'running'"
          @click="runRuntimeInstall"
        >
          <RefreshCw :size="14" />
          {{ runtimeInstallLabel }}
        </button>
        <button
          data-action="select-pi-web-directory"
          type="button"
          :disabled="runtimeBusy || installStatus === 'running'"
          @click="pickRuntimeDirectory"
        >
          手动指定目录
        </button>
        <button
          v-if="runtime?.source === 'custom'"
          data-action="reset-pi-web-directory"
          type="button"
          :disabled="runtimeBusy || installStatus === 'running'"
          @click="resetRuntimeDirectory"
        >
          取消指定
        </button>
        <button
          v-if="runtime?.canClean"
          data-action="clean-pi-web-runtime"
          type="button"
          :disabled="runtimeBusy || installStatus === 'running'"
          @click="cleanRuntime"
        >
          <Trash2 :size="14" />
          清理
        </button>
      </div>

      <p
        v-if="runtimeMessage || runtimeError"
        class="pi-web-config-progress"
      >
        {{ runtimeError || runtimeMessage }}
      </p>
    </section>

    <section
      class="pi-web-diagnostics pi-web-config-card"
      :data-repair-needed="configStatus?.needsRepair ? 'true' : 'false'"
      aria-live="polite"
    >
      <div class="pi-web-card-heading">
        <div>
          <h3>PI-WEB 一键配置</h3>
          <p class="pi-web-health-title">
            直接填写 URL 和 Key，一键写入 models.json、settings.json 和 auth.json。
          </p>
        </div>
        <button
          data-action="check-pi-web-config"
          type="button"
          :disabled="configBusy || configRepairBusy"
          @click="refreshConfigStatus"
        >
          <RefreshCw :size="14" />
          {{ configBusy ? '检查中' : '重新检测' }}
        </button>
      </div>

      <p class="pi-web-config-meta">
        当前写入：{{ configModelLabel }}
      </p>
      <p v-if="configStatus?.agentDir">
        配置目录：{{ configStatus.agentDir }}
      </p>

      <div
        v-if="configStatus"
        class="pi-web-config-grid"
      >
        <span :data-ready="configStatus.settingsExists">settings.json</span>
        <span :data-ready="configStatus.modelsExists">models.json</span>
        <span :data-ready="configStatus.authExists">auth.json</span>
        <span :data-ready="configStatus.authConfigured">API Key</span>
      </div>

      <label class="pi-web-key-field">
        <span>URL 地址</span>
        <input
          v-model="configBaseUrl"
          data-field="pi-web-base-url"
          type="url"
          autocomplete="off"
          spellcheck="false"
          placeholder="https://ai.leihuo.netease.com/v1"
          :disabled="configRepairBusy"
        >
      </label>

      <label class="pi-web-key-field">
        <span>API Key</span>
        <input
          v-model="configApiKey"
          data-field="pi-web-api-key"
          type="password"
          autocomplete="off"
          spellcheck="false"
          placeholder="请输入当前用户自己的 API Key"
          :disabled="configRepairBusy"
        >
      </label>

      <div class="pi-web-card-actions">
        <button
          data-action="repair-pi-web-config"
          type="button"
          :disabled="!canRepairConfig"
          @click="repairConfig"
        >
          <RefreshCw :size="14" />
          {{ configRepairBusy ? '配置中' : '一键配置' }}
        </button>
      </div>

      <p class="pi-web-config-progress">
        {{ configProgressText }}
      </p>
    </section>

    <section
      v-if="chatHealth"
      class="pi-web-diagnostics"
      :data-health-state="chatHealth.state"
      aria-live="polite"
    >
      <h3>对话检测</h3>
      <p class="pi-web-health-title">
        {{ chatHealth.message }}
      </p>
      <p v-if="chatHealth.provider || chatHealth.modelId">
        当前模型：{{ chatHealth.provider || '未知提供商' }} / {{ chatHealth.modelId || '未知模型' }}
      </p>
      <p v-if="chatHealth.detail">
        {{ chatHealth.detail }}
      </p>
      <div
        v-if="canRepairDeveloperRole"
        class="pi-web-card-actions"
      >
        <button
          data-action="repair-pi-web-model"
          type="button"
          :disabled="repairBusy"
          @click="repairModelCompatibility"
        >
          <RefreshCw :size="14" />
          {{ repairBusy ? '修复中' : '修复模型兼容' }}
        </button>
      </div>
      <p
        v-if="repairResult"
        class="pi-web-repair-result"
      >
        {{ repairResult.message }}。{{ repairResult.detail }}
      </p>
    </section>

    <section
      v-if="status?.detail || status?.missingDependency || status?.installLinks.length"
      class="pi-web-diagnostics"
      aria-live="polite"
    >
      <h3>诊断信息</h3>
      <p v-if="status.missingDependency">
        缺少 {{ status.missingDependency }}。请按下面链接安装后再重试。
      </p>
      <p v-if="status.detail">
        {{ status.detail }}
      </p>
      <div
        v-if="status.installLinks.length"
        class="pi-web-links"
      >
        <a
          v-for="link in status.installLinks"
          :key="link.url"
          :href="link.url"
          :data-install-link="status.missingDependency || link.label"
          target="_blank"
          rel="noreferrer"
        >
          {{ link.label }}
        </a>
      </div>
    </section>
  </main>

  <MediaToolProgressDialog
    :open="installDialogOpen"
    title="一键下载配置 PI-WEB"
    description="自动下载最新版 PI-WEB、写入 API 配置并校验，期间请不要关闭 Banana Box。"
    :progress="installProgress"
    :message="installMessageText"
    :logs="installLogs"
    :status="installStatus"
    :error="installError"
    @close="closeInstallDialog"
    @retry="runRuntimeInstall"
  />
</template>

<style scoped>
.pi-web-page {
  min-height: 100%;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px 22px;
  overflow-y: auto;
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.08), transparent 34%),
    linear-gradient(180deg, rgba(9, 20, 30, 0.74), rgba(5, 13, 20, 0.9));
}

.pi-web-header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--bb-border);
}

.pi-web-header p,
.pi-web-header h2 {
  margin: 0;
}

.pi-web-header p {
  color: var(--bb-primary);
  font: 11px var(--bb-mono);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.pi-web-header h2 {
  margin-top: 3px;
  font-size: 20px;
}

.pi-web-state {
  max-width: 132px;
  overflow: hidden;
  padding: 5px 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  color: var(--bb-primary-strong);
  background: var(--bb-primary-soft);
  font: 11px var(--bb-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pi-web-hero,
.pi-web-diagnostics {
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(180deg, rgba(18, 33, 45, 0.9), rgba(8, 19, 29, 0.88)),
    var(--bb-surface-soft);
  box-shadow: var(--bb-shadow-sm);
}

.pi-web-hero {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 14px;
  align-items: center;
  padding: 14px;
}

.pi-web-copy {
  min-width: 0;
  display: grid;
  gap: 5px;
}

.pi-web-copy strong {
  overflow-wrap: anywhere;
  font-size: 15px;
}

.pi-web-copy span {
  overflow-wrap: anywhere;
  color: var(--bb-text-soft);
  font: 12px var(--bb-mono);
}

.pi-web-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.pi-web-actions button {
  display: inline-flex;
  min-height: 32px;
  align-items: center;
  gap: 6px;
  padding: 0 11px;
  white-space: nowrap;
}

.pi-web-primary {
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary);
  color: #06231f;
  font-weight: 750;
}

.pi-web-diagnostics {
  max-height: 260px;
  overflow: auto;
  padding: 12px;
}

.pi-web-diagnostics[data-health-state="ok"] {
  border-color: rgba(102, 247, 211, 0.42);
}

.pi-web-diagnostics[data-health-state="error"] {
  border-color: rgba(255, 107, 107, 0.55);
}

.pi-web-diagnostics h3 {
  margin: 0 0 8px;
  font-size: 13px;
}

.pi-web-diagnostics p {
  margin: 0 0 8px;
  color: var(--bb-text-muted);
  line-height: 1.55;
  overflow-wrap: anywhere;
}

.pi-web-health-title {
  color: var(--bb-text);
  font-weight: 700;
}

.pi-web-card-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 8px;
}

.pi-web-card-heading h3,
.pi-web-card-heading p {
  margin: 0;
}

.pi-web-card-heading button {
  display: inline-flex;
  min-height: 30px;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
}

.pi-web-config-card[data-repair-needed="true"] {
  border-color: rgba(255, 214, 102, 0.42);
}

.pi-web-config-card {
  max-height: 420px;
  scrollbar-gutter: stable;
}

.pi-web-config-meta {
  font: 12px var(--bb-mono);
}

.pi-web-config-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
  margin: 8px 0;
}

.pi-web-config-grid span {
  min-width: 0;
  overflow: hidden;
  padding: 5px 7px;
  border: 1px solid rgba(255, 107, 107, 0.36);
  border-radius: var(--bb-radius-sm);
  color: var(--bb-text-muted);
  background: rgba(255, 107, 107, 0.08);
  font: 11px var(--bb-mono);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pi-web-config-grid span[data-ready="true"] {
  border-color: rgba(102, 247, 211, 0.36);
  color: var(--bb-primary-strong);
  background: var(--bb-primary-soft);
}

.pi-web-key-field {
  display: grid;
  gap: 6px;
  margin-top: 10px;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.pi-web-key-field input {
  min-height: 34px;
  width: 100%;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-sm);
  padding: 0 10px;
  color: var(--bb-text);
  background: rgba(5, 13, 20, 0.82);
  outline: none;
}

.pi-web-key-field input:focus-visible {
  border-color: rgba(102, 247, 211, 0.62);
  box-shadow: 0 0 0 2px rgba(102, 247, 211, 0.12);
}

.pi-web-key-field input:disabled {
  opacity: 0.58;
}

.pi-web-links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.pi-web-card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}

.pi-web-card-actions button {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  border: 1px solid rgba(102, 247, 211, 0.42);
  border-radius: var(--bb-radius-sm);
  color: #06231f;
  background: var(--bb-primary);
  font-weight: 700;
}

.pi-web-card-actions button:disabled {
  cursor: not-allowed;
  opacity: 0.52;
}

.pi-web-config-progress {
  margin-top: 10px;
  padding: 8px 10px;
  border: 1px solid rgba(102, 247, 211, 0.24);
  border-radius: var(--bb-radius-sm);
  color: var(--bb-primary-strong);
  background: rgba(102, 247, 211, 0.08);
}

.pi-web-repair-result {
  margin-top: 10px;
  color: var(--bb-primary-strong);
}

.pi-web-links a {
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  padding: 0 10px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-sm);
  color: var(--bb-primary-strong);
  background: var(--bb-primary-soft);
  text-decoration: none;
}

@media (max-width: 660px) {
  .pi-web-page {
    padding: 14px 12px;
  }

  .pi-web-hero {
    grid-template-columns: 1fr;
  }

  .pi-web-actions {
    justify-content: flex-start;
  }

  .pi-web-card-heading {
    display: grid;
  }

  .pi-web-config-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>

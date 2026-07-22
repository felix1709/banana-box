<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Activity, ExternalLink, Play, RefreshCw, Square } from '@lucide/vue'
import {
  getPiWebChatHealth,
  getPiWebStatus,
  openPiWeb,
  repairPiWebModelCompatibility,
  startPiWeb,
  stopPiWeb,
  type PiWebChatHealth,
  type PiWebRepairResult,
  type PiWebStatus,
} from '@/lib/piWebIpc'

const status = ref<PiWebStatus | null>(null)
const chatHealth = ref<PiWebChatHealth | null>(null)
const repairResult = ref<PiWebRepairResult | null>(null)
const busy = ref(false)
const healthBusy = ref(false)
const repairBusy = ref(false)

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

onMounted(refresh)
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
}
</style>

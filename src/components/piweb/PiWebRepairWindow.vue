<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RefreshCw } from '@lucide/vue'
import {
  getPiWebConfigStatus,
  repairPiWebConfig,
  type PiWebConfigRepairResult,
  type PiWebConfigStatus,
} from '@/lib/piWebIpc'

const configStatus = ref<PiWebConfigStatus | null>(null)
const configRepairResult = ref<PiWebConfigRepairResult | null>(null)
const configApiKey = ref('')
const configBusy = ref(false)
const configRepairBusy = ref(false)

const configModelLabel = computed(() => {
  const provider = configStatus.value?.defaultProvider || '雷火'
  const model = configStatus.value?.defaultModel || 'glm-5.2'
  return `${provider} / ${model}`
})

const canRepairConfig = computed(() => {
  return Boolean(configApiKey.value.trim() && !configRepairBusy.value)
})

async function refreshConfigStatus() {
  configBusy.value = true
  try {
    configStatus.value = await getPiWebConfigStatus()
  } finally {
    configBusy.value = false
  }
}

async function repairConfig() {
  if (!canRepairConfig.value) return
  configRepairBusy.value = true
  try {
    configRepairResult.value = await repairPiWebConfig(configApiKey.value)
    configStatus.value = configRepairResult.value.status
  } finally {
    configApiKey.value = ''
    configRepairBusy.value = false
  }
}

onMounted(() => {
  void refreshConfigStatus()
})
</script>

<template>
  <main class="pi-web-repair-window">
    <header class="pi-web-repair-header">
      <div>
        <p>PI-Web</p>
        <h2>配置修复</h2>
      </div>
      <button
        data-action="check-pi-web-config"
        type="button"
        :disabled="configBusy"
        @click="refreshConfigStatus"
      >
        <RefreshCw :size="14" />
        {{ configBusy ? '检测中' : '重新检测' }}
      </button>
    </header>

    <section
      class="pi-web-repair-body"
      :data-repair-needed="configStatus?.needsRepair ? 'true' : 'false'"
      aria-live="polite"
    >
      <div class="repair-summary">
        <strong>{{ configStatus?.message || '正在检测 PI-Web 配置' }}</strong>
        <span>当前模型：{{ configModelLabel }}</span>
      </div>

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

      <p v-if="configStatus?.detail">
        {{ configStatus.detail }}
      </p>

      <label class="pi-web-key-field">
        <span>PI-Web API Key</span>
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
          {{ configRepairBusy ? '修复中' : '一键修复' }}
        </button>
      </div>

      <p
        v-if="configRepairResult"
        class="pi-web-repair-result"
      >
        {{ configRepairResult.message }}。{{ configRepairResult.detail }}
      </p>
    </section>
  </main>
</template>

<style scoped>
.pi-web-repair-window {
  width: 100vw;
  height: 100vh;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--bb-text);
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.08), transparent 34%),
    linear-gradient(180deg, rgba(9, 20, 30, 0.92), rgba(5, 13, 20, 0.98));
}

.pi-web-repair-header {
  flex: 0 0 auto;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 18px 20px 14px;
  border-bottom: 1px solid var(--bb-border);
}

.pi-web-repair-header p,
.pi-web-repair-header h2 {
  margin: 0;
}

.pi-web-repair-header p {
  color: var(--bb-primary);
  font: 11px var(--bb-mono);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.pi-web-repair-header h2 {
  margin-top: 3px;
  font-size: 18px;
}

.pi-web-repair-header button,
.pi-web-card-actions button {
  display: inline-flex;
  min-height: 32px;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  white-space: nowrap;
}

.pi-web-repair-body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 16px 20px 20px;
  scrollbar-gutter: stable;
}

.repair-summary {
  display: grid;
  gap: 5px;
  padding: 12px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface-soft);
}

.repair-summary strong {
  overflow-wrap: anywhere;
  font-size: 14px;
}

.repair-summary span,
.pi-web-repair-body p {
  color: var(--bb-text-muted);
  font-size: 12px;
  line-height: 1.55;
  overflow-wrap: anywhere;
}

.pi-web-config-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 7px;
  margin: 12px 0;
}

.pi-web-config-grid span {
  min-width: 0;
  overflow: hidden;
  padding: 6px 8px;
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
  margin-top: 12px;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.pi-web-key-field input {
  min-height: 34px;
  width: 100%;
  min-width: 0;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-sm);
  padding: 0 10px;
  color: var(--bb-text);
  background: rgba(5, 13, 20, 0.82);
  outline: none;
}

.pi-web-key-field input:focus-visible {
  border-color: rgba(102, 247, 211, 0.62);
  box-shadow: var(--bb-focus);
}

.pi-web-card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.pi-web-card-actions button {
  border-color: rgba(102, 247, 211, 0.42);
  color: #06231f;
  background: var(--bb-primary);
  font-weight: 700;
}

.pi-web-repair-result {
  margin-top: 12px;
  color: var(--bb-primary-strong);
}

@media (max-width: 520px) {
  .pi-web-repair-header {
    display: grid;
  }

  .pi-web-config-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>

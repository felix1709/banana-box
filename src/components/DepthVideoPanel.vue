<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import {
  convertVideoToDepthVideo,
  prepareDepthVideoEngine,
  suggestDepthVideoOutputPath,
} from '@/lib/ipc'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const sourcePath = ref(ui.depthVideoSourcePath)
const outputPath = ref('')
const loading = ref(false)
const error = ref('')
const progress = ref(0)
const progressText = ref('')
const depthEngineStorageKey = 'banana-box-depth-video-engine'
const enginePath = ref(window.localStorage.getItem(depthEngineStorageKey) ?? '')
const preparingEngine = ref(false)
const engineSetupMessage = ref('')

const fileName = computed(() => {
  if (!sourcePath.value) return ''
  return sourcePath.value.split(/[\\/]/).pop() ?? sourcePath.value
})

const engineName = computed(() => {
  if (!enginePath.value) return 'banana-depth-video.exe'
  return enginePath.value.split(/[\\/]/).pop() ?? enginePath.value
})

watch(
  () => ui.depthVideoSourcePath,
  (nextPath) => {
    if (!nextPath) return
    sourcePath.value = nextPath
    outputPath.value = ''
    error.value = ''
    resetProgress()
  },
)

function resetProgress() {
  progress.value = 0
  progressText.value = ''
}

function depthVideoErrorMessage(reason: unknown) {
  const raw = reason instanceof Error ? reason.message : String(reason)
  if (raw.includes('DEPTH_VIDEO_ENGINE_MISSING')) {
    return '本地深度视频引擎不可用。请先选择 banana-depth-video.exe，或把 banana-depth-video 加入系统 PATH 后重试。'
  }
  if (raw.includes('PYTHON_NOT_FOUND')) {
    return '未找到 Python。请先安装 Python 3.10+，并勾选 Add python.exe to PATH。'
  }
  if (raw.includes('DEPTH_VIDEO_ENGINE_SETUP_FAILED')) {
    return raw.replace('DEPTH_VIDEO_ENGINE_SETUP_FAILED', '本地深度视频引擎配置失败：').trim()
  }
  if (raw.trim()) return raw
  return '深度视频转换失败，请确认视频文件可用后重试。'
}

async function prepareEngine() {
  if (preparingEngine.value) return
  preparingEngine.value = true
  error.value = ''
  engineSetupMessage.value = '正在下载官方 Small 模型并配置本地环境，首次可能需要较长时间。'
  try {
    const result = await prepareDepthVideoEngine()
    enginePath.value = result.enginePath
    window.localStorage.setItem(depthEngineStorageKey, result.enginePath)
    engineSetupMessage.value = result.message
  } catch (reason) {
    error.value = depthVideoErrorMessage(reason)
    engineSetupMessage.value = ''
  } finally {
    preparingEngine.value = false
  }
}

async function pickDepthEngine() {
  const picked = await open({
    multiple: false,
    filters: [
      {
        name: '本地深度视频引擎',
        extensions: ['exe', 'cmd', 'bat', 'ps1'],
      },
    ],
  })
  if (!picked || Array.isArray(picked)) return
  enginePath.value = picked
  window.localStorage.setItem(depthEngineStorageKey, picked)
  error.value = ''
}

async function pickFile() {
  const picked = await open({
    multiple: false,
    filters: [
      {
        name: '视频文件',
        extensions: ['mp4', 'mov', 'webm', 'avi', 'mkv'],
      },
    ],
  })
  if (!picked || Array.isArray(picked)) return
  sourcePath.value = picked
  outputPath.value = ''
  error.value = ''
  resetProgress()
}

async function onConvert() {
  if (!sourcePath.value || loading.value) return
  loading.value = true
  outputPath.value = ''
  error.value = ''
  progress.value = 12
  progressText.value = '准备本地转换'
  try {
    const defaultPath = await suggestDepthVideoOutputPath({
      sourcePath: sourcePath.value,
    })
    progress.value = 26
    progressText.value = '选择保存位置'

    const output = await save({
      defaultPath,
      filters: [{ name: '视频文件', extensions: ['mp4'] }],
    })
    if (!output) {
      resetProgress()
      return
    }

    progress.value = 66
    progressText.value = '转换中'
    const result = await convertVideoToDepthVideo({
      sourcePath: sourcePath.value,
      outputPath: output,
      ...(enginePath.value.trim() ? { enginePath: enginePath.value.trim() } : {}),
    })
    outputPath.value = result.outputPath
    progress.value = 100
    progressText.value = '转换完成'
  } catch (reason) {
    error.value = depthVideoErrorMessage(reason)
    resetProgress()
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section class="tool-panel">
    <section class="engine-card">
      <div>
        <strong>本地深度视频引擎</strong>
        <p>
          当前使用：{{ engineName }}
        </p>
      </div>
      <div class="engine-actions">
        <button
          type="button"
          class="pick-depth-engine-button"
          @click="pickDepthEngine"
        >
          选择引擎
        </button>
        <button
          type="button"
          class="prepare-depth-engine-button"
          :disabled="preparingEngine || loading"
          @click="prepareEngine"
        >
          {{ preparingEngine ? '配置中...' : '下载并配置' }}
        </button>
      </div>
    </section>
    <p
      v-if="engineSetupMessage"
      class="engine-setup-message"
    >
      {{ engineSetupMessage }}
    </p>

    <div class="upload-zone">
      <strong>深度视频转换</strong>
      <p>导入视频后，使用本地深度估计引擎生成深度图视频并另存为。</p>
      <button
        type="button"
        class="pick-depth-video-button"
        @click="pickFile"
      >
        导入视频
      </button>
      <p
        v-if="fileName"
        class="file-name"
      >
        {{ fileName }}
      </p>
    </div>

    <button
      type="button"
      class="convert-depth-video-button"
      :disabled="!sourcePath || loading"
      @click="onConvert"
    >
      {{ loading ? '转换中...' : '开始转换' }}
    </button>

    <div
      v-if="loading || progress > 0"
      class="progress-panel"
    >
      <div
        class="progress-bar"
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="progress"
      >
        <div
          class="progress-fill"
          :style="{ width: `${progress}%` }"
        />
      </div>
      <span class="progress-text">{{ progressText || '转换中' }}</span>
    </div>

    <p
      v-if="outputPath"
      class="status"
    >
      已输出：{{ outputPath }}
    </p>
    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>
  </section>
</template>

<style scoped>
.tool-panel {
  min-height: 100%;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.08), transparent 35%),
    linear-gradient(180deg, rgba(9, 20, 30, 0.74), rgba(5, 13, 20, 0.9));
}

.upload-zone {
  min-height: 190px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  border: 1px dashed rgba(102, 247, 211, 0.38);
  border-radius: var(--bb-radius-lg);
  background:
    radial-gradient(circle at 50% 0%, rgba(102, 247, 211, 0.1), transparent 42%),
    linear-gradient(180deg, rgba(18, 33, 45, 0.9), rgba(8, 19, 29, 0.88)),
    var(--bb-surface-soft);
  color: var(--bb-text);
  text-align: center;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.engine-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 11px;
  border: 1px solid rgba(102, 247, 211, 0.18);
  border-radius: var(--bb-radius-md);
  background: rgba(11, 25, 36, 0.78);
}

.engine-card strong {
  display: block;
  color: var(--bb-text);
}

.engine-card p {
  margin: 4px 0 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  overflow-wrap: anywhere;
}

.engine-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 8px;
  align-items: center;
}

.pick-depth-engine-button {
  flex: 0 0 auto;
}

.prepare-depth-engine-button {
  flex: 0 0 auto;
  border-color: rgba(102, 247, 211, 0.42);
  background: rgba(102, 247, 211, 0.12);
}

.engine-setup-message {
  margin: -2px 2px 2px;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.upload-zone p,
.status {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 13px;
}

.file-name,
.status,
.error {
  overflow-wrap: anywhere;
}

.convert-depth-video-button {
  align-self: flex-start;
  border-color: rgba(102, 247, 211, 0.55);
  background: linear-gradient(180deg, var(--bb-primary-strong), var(--bb-primary));
  color: #041017;
  font-weight: 600;
  box-shadow: 0 0 24px rgba(102, 247, 211, 0.18);
}

.progress-panel {
  display: flex;
  align-items: center;
  gap: 8px;
}

.progress-bar {
  width: min(320px, 100%);
  height: 10px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(5, 14, 22, 0.76);
  box-shadow: inset 0 0 0 1px rgba(102, 247, 211, 0.1);
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--bb-primary), #68c9ff);
  box-shadow: 0 0 18px rgba(102, 247, 211, 0.28);
  transition: width 160ms ease;
}

.progress-text {
  min-width: 64px;
  color: var(--bb-text-muted);
  font-size: 12px;
}

.error {
  margin: 0;
  color: var(--bb-danger);
  font-size: 13px;
}
</style>

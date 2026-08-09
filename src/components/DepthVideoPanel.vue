<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { convertVideoToDepthVideo, suggestDepthVideoOutputPath } from '@/lib/ipc'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const sourcePath = ref(ui.depthVideoSourcePath)
const outputPath = ref('')
const loading = ref(false)
const error = ref('')
const progress = ref(0)
const progressText = ref('')

const fileName = computed(() => {
  if (!sourcePath.value) return ''
  return sourcePath.value.split(/[\\/]/).pop() ?? sourcePath.value
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
    return '本地深度视频引擎不可用。请先安装或配置本地 Depth Anything 转换引擎后重试。'
  }
  if (raw.trim()) return raw
  return '深度视频转换失败，请确认视频文件可用后重试。'
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

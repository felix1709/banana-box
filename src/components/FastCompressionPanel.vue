<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { compressMedia, suggestCompressedOutputPath } from '@/lib/ipc'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const sourcePath = ref(ui.compressionSourcePath)
const targetMb = ref(10)
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
  () => ui.compressionSourcePath,
  (nextPath) => {
    if (!nextPath) return
    sourcePath.value = nextPath
    outputPath.value = ''
    error.value = ''
    progress.value = 0
    progressText.value = ''
  },
)

function resetProgress() {
  progress.value = 0
  progressText.value = ''
}

function compressionErrorMessage(reason: unknown) {
  if (reason instanceof Error && reason.message.trim()) return reason.message
  if (typeof reason === 'string' && reason.trim()) return reason
  return '压缩失败，请确认文件可用后重试'
}

async function pickFile() {
  const picked = await open({
    multiple: false,
    filters: [
      {
        name: '媒体文件',
        extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'mp4', 'mov', 'webm', 'avi', 'mkv'],
      },
    ],
  })
  if (!picked || Array.isArray(picked)) return
  sourcePath.value = picked
  outputPath.value = ''
  error.value = ''
  resetProgress()
}

async function onCompress() {
  if (!sourcePath.value || targetMb.value <= 0) return
  loading.value = true
  error.value = ''
  outputPath.value = ''
  progress.value = 10
  progressText.value = '准备压缩'
  try {
    const defaultPath = await suggestCompressedOutputPath({
      sourcePath: sourcePath.value,
    })
    progress.value = 25
    progressText.value = '选择保存位置'

    const output = await save({ defaultPath })
    if (!output) {
      resetProgress()
      return
    }

    progress.value = 65
    progressText.value = '压缩中'
    const result = await compressMedia({
      sourcePath: sourcePath.value,
      targetMb: Number(targetMb.value),
      outputPath: output,
    })
    outputPath.value = result.outputPath
    progress.value = 100
    progressText.value = '压缩完成'
  } catch (reason) {
    error.value = compressionErrorMessage(reason)
    resetProgress()
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <section class="tool-panel">
    <div class="upload-zone">
      <strong>快速压缩</strong>
      <p>导入图片或视频，输入目标大小 MB 后另存为。</p>
      <button
        type="button"
        class="pick-file-button"
        @click="pickFile"
      >
        导入文件
      </button>
      <p
        v-if="fileName"
        class="file-name"
      >
        {{ fileName }}
      </p>
    </div>

    <label class="target-row">
      目标大小 MB
      <input
        v-model.number="targetMb"
        class="target-mb-input"
        type="number"
        min="0.1"
        step="0.1"
      >
    </label>

    <button
      type="button"
      class="compress-button"
      :disabled="!sourcePath || targetMb <= 0 || loading"
      @click="onCompress"
    >
      {{ loading ? '压缩中...' : '开始压缩' }}
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
      <span class="progress-text">{{ progressText || '压缩中' }}</span>
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
.status {
  overflow-wrap: anywhere;
}

.target-row {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--bb-text);
  font-size: 13px;
}

.target-mb-input {
  width: 96px;
  padding: 7px 8px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
}

.compress-button {
  align-self: flex-start;
  border-color: rgba(102, 247, 211, 0.55);
  background: linear-gradient(180deg, var(--bb-primary-strong), var(--bb-primary));
  color: #041017;
  font-weight: 600;
  box-shadow: 0 0 24px rgba(102, 247, 211, 0.18);
}
.compress-button:hover:not(:disabled) {
  border-color: var(--bb-primary-strong);
  background: linear-gradient(180deg, #c2fff2, #78ffdf);
}
.compress-button:disabled {
  border-color: var(--bb-border);
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
  overflow-wrap: anywhere;
}
</style>

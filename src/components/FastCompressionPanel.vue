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
  } catch {
    error.value = '压缩失败，请确认文件可用后重试'
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
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.upload-zone {
  min-height: 190px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 1px dashed #cbd5e1;
  border-radius: 6px;
  background: #f8fafc;
  color: #334155;
}

.upload-zone p,
.status {
  margin: 0;
  color: #64748b;
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
  color: #334155;
  font-size: 13px;
}

.target-mb-input {
  width: 96px;
  padding: 6px;
  border: 1px solid #d7dde7;
  border-radius: 4px;
}

.compress-button {
  align-self: flex-start;
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
  background: #e2e8f0;
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  background: #2563eb;
  transition: width 160ms ease;
}

.progress-text {
  min-width: 64px;
  color: #475569;
  font-size: 12px;
}

.error {
  margin: 0;
  color: #b91c1c;
  font-size: 13px;
}
</style>

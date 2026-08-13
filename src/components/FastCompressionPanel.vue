<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open, save } from '@tauri-apps/plugin-dialog'
import { compressMedia, prepareFfmpegTools, suggestCompressedOutputPath } from '@/lib/ipc'
import { revealOutputPath } from '@/lib/outputReveal'
import { useUiStore } from '@/stores/ui'
import MediaToolProgressDialog from '@/components/MediaToolProgressDialog.vue'

const ui = useUiStore()
const sourcePath = ref(ui.compressionSourcePath)
const targetMb = ref(10)
const outputPath = ref('')
const loading = ref(false)
const error = ref('')
const revealError = ref('')
const progress = ref(0)
const progressText = ref('')
const ffmpegDownloadUrl = 'https://ffmpeg.org/download.html'
const ffmpegMissing = computed(() => /ffmpeg|ffprobe/i.test(error.value))
const setupDialogOpen = ref(false)
const setupStatus = ref<'idle' | 'running' | 'success' | 'error'>('idle')
const setupProgress = ref(0)
const setupMessage = ref('')
const setupLogs = ref<string[]>([])
const setupError = ref('')
const setupOperationId = ref('')
let unlistenSetupProgress: UnlistenFn | null = null

interface MediaToolProgressPayload {
  operationId: string
  tool: string
  phase: string
  progress: number
  message: string
  detail?: string
  level: 'info' | 'success' | 'error'
}

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
    revealError.value = ''
    progress.value = 0
    progressText.value = ''
  },
)

function resetProgress() {
  progress.value = 0
  progressText.value = ''
}

function newOperationId(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function appendSetupLog(line: string) {
  setupLogs.value = [...setupLogs.value, line].slice(-80)
}

function applySetupProgress(payload: MediaToolProgressPayload) {
  if (payload.operationId !== setupOperationId.value || payload.tool !== 'ffmpeg') return
  setupProgress.value = Math.max(setupProgress.value, payload.progress)
  setupMessage.value = payload.message
  appendSetupLog(payload.detail ? `${payload.message}：${payload.detail}` : payload.message)
  if (payload.level === 'error') setupStatus.value = 'error'
  if (payload.level === 'success') setupStatus.value = 'success'
}

async function startSetupProgressListener() {
  unlistenSetupProgress?.()
  unlistenSetupProgress = await listen<MediaToolProgressPayload>('media-tool-progress', (event) => {
    applySetupProgress(event.payload)
  })
}

function closeSetupDialog() {
  if (setupStatus.value === 'running') return
  setupDialogOpen.value = false
}

onBeforeUnmount(() => {
  unlistenSetupProgress?.()
})

function compressionErrorMessage(reason: unknown) {
  const raw = reason instanceof Error ? reason.message : typeof reason === 'string' ? reason : ''
  if (/ffmpeg|ffprobe/i.test(raw)) {
    return '视频压缩组件还没配置好，请点击“一键配置 FFmpeg”。Banana Box 会自动下载到应用目录，不需要你手动设置 PATH。'
  }
  if (raw.trim()) return raw
  return '压缩失败，请确认文件可用后重试'
}

const ffmpegMissingTools = computed(() => {
  const missing = new Set<string>()
  if (/ffmpeg/i.test(error.value)) missing.add('ffmpeg')
  if (/ffprobe/i.test(error.value)) missing.add('ffprobe')
  return [...missing]
})

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
  revealError.value = ''
  resetProgress()
}

async function revealOutputFolder(path = outputPath.value) {
  if (!path) return
  revealError.value = ''
  const opened = await revealOutputPath(path)
  if (!opened) {
    revealError.value = '文件已生成，但无法自动打开文件夹。'
  }
}

async function onCompress() {
  if (!sourcePath.value || targetMb.value <= 0) return
  loading.value = true
  error.value = ''
  revealError.value = ''
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
    await revealOutputFolder(result.outputPath)
  } catch (reason) {
    error.value = compressionErrorMessage(reason)
    resetProgress()
  } finally {
    loading.value = false
  }
}

async function onPrepareFfmpeg() {
  if (setupStatus.value === 'running') return
  setupOperationId.value = newOperationId('ffmpeg')
  setupDialogOpen.value = true
  setupStatus.value = 'running'
  setupProgress.value = 5
  setupMessage.value = '准备配置 FFmpeg'
  setupLogs.value = ['开始配置应用内 FFmpeg，不需要手动设置 PATH']
  setupError.value = ''
  error.value = ''
  await startSetupProgressListener()
  try {
    const result = await prepareFfmpegTools({ operationId: setupOperationId.value })
    setupProgress.value = 100
    setupStatus.value = 'success'
    setupMessage.value = result.message
    appendSetupLog(`ffmpeg：${result.ffmpegPath}`)
    appendSetupLog(`ffprobe：${result.ffprobePath}`)
  } catch (reason) {
    setupStatus.value = 'error'
    setupProgress.value = Math.max(setupProgress.value, 10)
    setupError.value = compressionErrorMessage(reason)
    setupMessage.value = 'FFmpeg 配置失败'
    appendSetupLog(setupError.value)
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

    <section
      class="ffmpeg-setup-card dependency-help"
      aria-label="FFmpeg 配置"
    >
      <div>
        <strong>FFmpeg 视频压缩组件</strong>
        <p>
          使用视频压缩前可先一键配置。Banana Box 会自动下载 ffmpeg 和 ffprobe 到应用目录，不需要手动设置 PATH。
        </p>
      </div>
      <button
        type="button"
        class="prepare-ffmpeg-button"
        :disabled="setupStatus === 'running'"
        @click="onPrepareFfmpeg"
      >
        {{ setupStatus === 'running' ? '正在配置...' : '一键配置 FFmpeg' }}
      </button>
    </section>

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
      <button
        type="button"
        class="open-output-folder-button"
        @click="revealOutputFolder()"
      >
        打开所在文件夹
      </button>
    </p>
    <p
      v-if="revealError"
      class="reveal-error"
    >
      {{ revealError }}
    </p>
    <p
      v-if="error"
      class="error"
    >
      {{ error }}
    </p>

    <section
      v-if="ffmpegMissing"
      class="dependency-help"
      aria-label="视频压缩组件安装说明"
    >
      <strong>缺少视频压缩组件</strong>
      <p>
        视频压缩需要完整 FFmpeg，其中包含 ffmpeg 和 ffprobe。安装后请重新打开 Banana Box，或确认 FFmpeg 的 bin 目录已经加入 PATH。
      </p>
      <p v-if="ffmpegMissingTools.length">
        当前提示缺少：{{ ffmpegMissingTools.join('、') }}
      </p>
      <a
        :href="ffmpegDownloadUrl"
        data-install-link="ffmpeg"
        target="_blank"
        rel="noreferrer"
      >
        打开 FFmpeg 官方下载页
      </a>
    </section>

    <MediaToolProgressDialog
      :open="setupDialogOpen"
      title="配置 FFmpeg"
      description="Banana Box 会把 ffmpeg 和 ffprobe 下载到应用目录，不需要你手动设置 PATH。"
      :progress="setupProgress"
      :message="setupMessage"
      :logs="setupLogs"
      :status="setupStatus"
      :error="setupError"
      @close="closeSetupDialog"
      @retry="onPrepareFfmpeg"
    />
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
.status,
.reveal-error {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 13px;
}

.file-name,
.status,
.reveal-error {
  overflow-wrap: anywhere;
}

.status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.open-output-folder-button {
  min-height: 28px;
  padding: 0 9px;
  border-color: rgba(102, 247, 211, 0.38);
  background: rgba(102, 247, 211, 0.1);
  color: var(--bb-primary-strong);
  font-size: 12px;
}

.reveal-error {
  color: var(--bb-warning);
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

.dependency-help {
  display: grid;
  gap: 7px;
  padding: 11px;
  border: 1px solid rgba(255, 196, 87, 0.38);
  border-radius: var(--bb-radius-md);
  background: rgba(255, 196, 87, 0.08);
  color: var(--bb-text);
}

.dependency-help strong,
.dependency-help p {
  margin: 0;
}

.dependency-help p {
  color: var(--bb-text-muted);
  font-size: 13px;
  line-height: 1.5;
}

.dependency-help a {
  width: max-content;
  max-width: 100%;
  min-height: 30px;
  display: inline-flex;
  align-items: center;
  padding: 0 10px;
  border: 1px solid rgba(102, 247, 211, 0.42);
  border-radius: var(--bb-radius-sm);
  color: var(--bb-primary-strong);
  background: var(--bb-primary-soft);
  text-decoration: none;
}

.prepare-ffmpeg-button {
  width: max-content;
  max-width: 100%;
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
}
</style>

<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useProviderStore } from '@/stores/providers'
import { useUiStore } from '@/stores/ui'
import { importImageFromPath, readImageBytes, saveImage } from '@/lib/ipc'
import { reverseImagePrompt } from '@/lib/provider-ipc'
import type { AiProvider } from '@/types/providers'

const providers = useProviderStore()
const ui = useUiStore()
const fileInput = ref<HTMLInputElement | null>(null)
const imagePath = ref<string | null>(null)
const imageName = ref('')
const previewUrl = ref('')
const result = ref('')
const loading = ref(false)
const error = ref('')
const reverseImageProviderId = 'reverse-image'

function reverseImageErrorMessage(reason: unknown) {
  const code = typeof reason === 'string' ? reason : reason instanceof Error ? reason.message : ''
  return (
    {
      PROVIDER_HTTP_ERROR: '服务拒绝了图片请求，请检查模型、图片格式和尺寸',
      PROVIDER_REQUEST_FAILED: '网络请求失败，请检查网络连接或服务地址后重试',
      PROVIDER_TIMEOUT: '图片反推请求超时，请稍后重试',
      INVALID_PROVIDER_URL: '图片反推服务地址无效，请在设置中重新保存服务地址',
      PROVIDER_REDIRECT_FORBIDDEN: '图片反推服务发生跳转，请检查服务地址是否填写完整',
      PROVIDER_RESPONSE_TOO_LARGE: '服务返回内容过大，请换一张图片或稍后重试',
      PROVIDER_CREDENTIALS_REQUIRED: '未找到 API Key，请在设置中重新保存',
      PROVIDER_CREDENTIAL_STORE_UNAVAILABLE: '无法读取本机 API Key，请在设置中重新保存',
      INVALID_MODEL: '当前模型不可用，请在设置中重新检测并选择支持视觉的模型',
      IMAGE_TOO_LARGE: '图片超过 10MB，请压缩后再试',
      IMAGE_NOT_FOUND: '找不到已导入的图片，请重新选择图片',
      INVALID_IMAGE_PATH: '图片路径无效，请重新导入图片后再试',
      INVALID_PROVIDER_RESPONSE: '服务返回的内容无法识别，请更换支持视觉的模型后重试',
    }[code] ?? '反推失败，请检查 API 设置后重试'
  )
}

function reverseImageModel(provider: AiProvider | undefined) {
  if (!provider) return ''
  const models = provider.availableModels
  if (provider.defaultModel && (!models.length || models.includes(provider.defaultModel))) {
    return provider.defaultModel
  }
  if (provider.probedModel && (!models.length || models.includes(provider.probedModel))) {
    return provider.probedModel
  }
  return models[0] ?? provider.defaultModel ?? provider.probedModel ?? ''
}

onMounted(async () => {
  try {
    await providers.load('reverse-image')
  } catch {
    error.value = '读取图片反推服务失败，请打开设置后重试'
  }
})

function basename(path: string) {
  return path.split(/[\\/]/).pop() ?? path
}

async function refreshPreview() {
  if (!imagePath.value) {
    previewUrl.value = ''
    return
  }
  try {
    previewUrl.value = await readImageBytes(imagePath.value)
  } catch {
    previewUrl.value = ''
  }
}

async function setImage(path: string, name: string) {
  imagePath.value = path
  imageName.value = name
  result.value = ''
  error.value = ''
  await refreshPreview()
}

async function importSourcePath(sourcePath: string) {
  if (!sourcePath) return
  try {
    const importedPath = await importImageFromPath({ sourcePath })
    await setImage(importedPath, basename(sourcePath))
  } catch {
    error.value = '导入图片失败，请重新选择图片'
  } finally {
    ui.reverseImageSourcePath = ''
  }
}

watch(
  () => ui.reverseImageSourcePath,
  (sourcePath) => {
    void importSourcePath(sourcePath)
  },
  { immediate: true },
)

function openPicker() {
  fileInput.value?.click()
}

async function attachImage(file: File) {
  if (!file.type.startsWith('image/')) {
    error.value = '请选择图片文件'
    return
  }
  const ext = file.name.split('.').pop()?.toLowerCase() || 'png'
  const bytes = Array.from(new Uint8Array(await file.arrayBuffer()))
  const savedPath = await saveImage(bytes, ext)
  await setImage(savedPath, file.name)
}

async function onPickImage(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) await attachImage(file)
  if (fileInput.value) fileInput.value.value = ''
}

async function onDrop(e: DragEvent) {
  e.preventDefault()
  const file = e.dataTransfer?.files?.[0]
  if (file) await attachImage(file)
}

async function onPaste(e: ClipboardEvent) {
  const file = Array.from(e.clipboardData?.files ?? []).find((item) =>
    item.type.startsWith('image/'),
  )
  if (file) await attachImage(file)
}

async function onReverse() {
  if (!imagePath.value) return
  const provider = providers.byId(reverseImageProviderId)
  const model = reverseImageModel(provider)
  if (!model) {
    error.value = '图片反推服务尚未就绪，请先在设置中完成配置'
    return
  }

  loading.value = true
  error.value = ''
  try {
    const response = await reverseImagePrompt({
      providerId: reverseImageProviderId,
      model,
      imagePath: imagePath.value,
    })
    result.value = response.prompt
  } catch (reason) {
    error.value = reverseImageErrorMessage(reason)
  } finally {
    loading.value = false
  }
}

function saveToPromptEditor() {
  if (!result.value) return
  ui.openEditor(null, {
    content: result.value,
    image: imagePath.value,
  })
}

function clearReverseState() {
  imagePath.value = null
  imageName.value = ''
  previewUrl.value = ''
  result.value = ''
  error.value = ''
  ui.reverseImageSourcePath = ''
}
</script>

<template>
  <section class="tool-panel">
    <div
      class="upload-zone"
      tabindex="0"
      @dragover.prevent
      @drop.prevent="onDrop"
      @paste="onPaste"
    >
      <strong>反推图片</strong>
      <p>点击导入、粘贴或拖拽图片到这里。</p>
      <div
        v-if="previewUrl"
        class="preview-frame"
      >
        <img
          class="image-preview"
          :src="previewUrl"
          :alt="imageName || 'preview'"
        >
      </div>
      <button
        type="button"
        @click="openPicker"
      >
        导入图片
      </button>
      <input
        ref="fileInput"
        class="file-input"
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        @change="onPickImage"
      >
      <p
        v-if="imageName"
        class="file-name"
      >
        {{ imageName }}
      </p>
    </div>

    <div class="reverse-actions">
      <button
        type="button"
        class="reverse-button"
        :disabled="!imagePath || loading"
        @click="onReverse"
      >
        {{ loading ? '反推中...' : '开始反推' }}
      </button>
      <button
        type="button"
        class="clear-button"
        data-action="clear-reverse"
        :disabled="loading || (!imagePath && !result)"
        @click="clearReverseState"
      >
        清空
      </button>
      <span
        v-if="error"
        class="error"
      >{{ error }}</span>
    </div>

    <textarea
      v-model="result"
      class="reverse-result"
      placeholder="反推结果会显示在这里"
      rows="6"
    />

    <button
      type="button"
      class="save-result-button"
      :disabled="!result"
      @click="saveToPromptEditor"
    >
      保存到提示词库
    </button>
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
  min-height: 184px;
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

.upload-zone:focus-visible {
  border-color: var(--bb-primary);
  box-shadow: var(--bb-focus);
}

.upload-zone p {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 13px;
}

.preview-frame {
  width: min(280px, 100%);
  height: 128px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.86);
  box-shadow: var(--bb-shadow-sm);
}

.image-preview {
  max-width: 100%;
  max-height: 100%;
  width: auto;
  height: auto;
  object-fit: contain;
}

.file-input {
  display: none;
}

.file-name {
  max-width: 100%;
  overflow-wrap: anywhere;
  color: var(--bb-text-muted);
}

.reverse-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.clear-button {
  min-width: 72px;
}
.reverse-button {
  border-color: rgba(102, 247, 211, 0.55);
  background: linear-gradient(180deg, var(--bb-primary-strong), var(--bb-primary));
  color: #041017;
  font-weight: 600;
  box-shadow: 0 0 24px rgba(102, 247, 211, 0.18);
}
.reverse-button:hover:not(:disabled) {
  border-color: var(--bb-primary-strong);
  background: linear-gradient(180deg, #c2fff2, #78ffdf);
}
.reverse-button:disabled {
  border-color: var(--bb-border);
}

.reverse-result {
  width: 100%;
  resize: vertical;
  min-height: 104px;
  box-sizing: border-box;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  padding: 8px;
  font-size: 13px;
  line-height: 1.45;
  background: rgba(5, 14, 22, 0.82);
  box-shadow: inset 0 0 28px rgba(102, 247, 211, 0.03);
}

.error {
  min-width: 0;
  color: var(--bb-danger);
  font-size: 12px;
  overflow-wrap: anywhere;
}
</style>

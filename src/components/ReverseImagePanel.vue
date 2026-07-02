<script setup lang="ts">
import { ref, watch } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { importImageFromPath, readImageBytes, reverseImagePrompt, saveImage } from '@/lib/ipc'

const lib = useLibraryStore()
const ui = useUiStore()
const fileInput = ref<HTMLInputElement | null>(null)
const imagePath = ref<string | null>(null)
const imageName = ref('')
const previewUrl = ref('')
const result = ref('')
const loading = ref(false)
const error = ref('')

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
  loading.value = true
  error.value = ''
  try {
    const response = await reverseImagePrompt({
      baseUrl: lib.library.settings.apiBaseUrl,
      apiKey: lib.library.settings.apiKey,
      model: lib.library.settings.reverseModel,
      imagePath: imagePath.value,
    })
    result.value = response.prompt
  } catch {
    error.value = '反推失败，请检查 API 设置后重试'
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
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.upload-zone {
  min-height: 220px;
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

.upload-zone p {
  margin: 0;
  color: #64748b;
  font-size: 13px;
}

.preview-frame {
  width: min(280px, 100%);
  height: 128px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border: 1px solid #d7dde7;
  border-radius: 6px;
  background: #fff;
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
}

.reverse-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.clear-button {
  min-width: 72px;
}

.reverse-result {
  width: 100%;
  resize: vertical;
  min-height: 120px;
  box-sizing: border-box;
  border: 1px solid #d7dde7;
  border-radius: 6px;
  padding: 8px;
  font-size: 13px;
  line-height: 1.45;
}

.error {
  color: #b91c1c;
  font-size: 12px;
}
</style>

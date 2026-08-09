<script setup lang="ts">
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()

function openCompression() {
  if (!ui.floatingActionFile) return
  ui.openCompressionWithSource(ui.floatingActionFile.filePath)
  ui.closeFloatingActionDialog()
}

function openReverseImage() {
  if (!ui.floatingActionFile) return
  ui.openReverseImageWithSource(ui.floatingActionFile.filePath)
  ui.closeFloatingActionDialog()
}

function openDepthVideo() {
  if (!ui.floatingActionFile) return
  ui.openDepthVideoWithSource(ui.floatingActionFile.filePath)
  ui.closeFloatingActionDialog()
}
</script>

<template>
  <div
    v-if="ui.floatingActionDialogOpen && ui.floatingActionFile"
    class="mask"
    @click.self="ui.closeFloatingActionDialog()"
  >
    <section class="dialog">
      <header class="header">
        <div>
          <h3>选择操作</h3>
          <p>{{ ui.floatingActionFile.fileName }}</p>
        </div>
        <button
          type="button"
          class="close-button"
          @click="ui.closeFloatingActionDialog()"
        >
          关闭
        </button>
      </header>

      <div class="action-list">
        <template v-if="ui.floatingActionFile.fileType === 'image'">
          <button
            type="button"
            class="action-button"
            data-action="reverse-image"
            @click="openReverseImage"
          >
            <strong>反推提示词</strong>
            <span>调用视觉模型生成可编辑提示词</span>
          </button>
          <button
            type="button"
            class="action-button"
            data-action="compress-image"
            @click="openCompression"
          >
            <strong>压缩图片</strong>
            <span>输入目标 MB 后另存为</span>
          </button>
        </template>

        <template v-else>
          <button
            type="button"
            class="action-button"
            data-action="compress-video"
            @click="openCompression"
          >
            <strong>压缩视频</strong>
            <span>输入目标 MB 后使用 FFmpeg 压缩</span>
          </button>
          <button
            type="button"
            class="action-button"
            data-action="convert-depth-video"
            @click="openDepthVideo"
          >
            <strong>转换深度视频</strong>
            <span>使用本地引擎生成深度图视频并另存为</span>
          </button>
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.68);
  backdrop-filter: blur(5px);
}

.dialog {
  width: 360px;
  max-width: calc(100vw - 24px);
  padding: 15px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-lg);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-dialog);
}

.header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-start;
  margin-bottom: 12px;
}

.header h3 {
  margin: 0;
  font-size: 16px;
  color: var(--bb-text);
}

.header p {
  margin: 4px 0 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  overflow-wrap: anywhere;
}

.close-button {
  flex: 0 0 auto;
}

.action-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.action-button {
  min-height: 58px;
  padding: 10px 11px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-md);
  background:
    linear-gradient(180deg, rgba(20, 35, 47, 0.96), rgba(9, 21, 31, 0.96));
  color: var(--bb-text);
  cursor: pointer;
  text-align: left;
  box-shadow: var(--bb-shadow-sm);
}

.action-button:hover {
  border-color: rgba(123, 255, 226, 0.34);
  background: var(--bb-primary-soft);
  box-shadow: 0 0 24px rgba(102, 247, 211, 0.09);
}

.action-button:disabled {
  cursor: not-allowed;
  color: var(--bb-text-soft);
  background: var(--bb-surface-muted);
  box-shadow: none;
}

.action-button strong,
.action-button span {
  display: block;
}

.action-button span {
  margin-top: 3px;
  color: var(--bb-text-muted);
  font-size: 12px;
}
</style>

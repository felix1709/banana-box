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
            data-action="reverse-video"
            disabled
          >
            <strong>视频反推</strong>
            <span>后续开发，当前先保留入口</span>
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
  background: rgba(15, 23, 42, 0.28);
}

.dialog {
  width: 360px;
  max-width: calc(100vw - 24px);
  padding: 14px;
  border: 1px solid #d7dde7;
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 16px 36px rgba(15, 23, 42, 0.22);
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
}

.header p {
  margin: 4px 0 0;
  color: #64748b;
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
  padding: 9px 10px;
  border: 1px solid #d7dde7;
  border-radius: 6px;
  background: #fff;
  color: #1f2937;
  cursor: pointer;
  text-align: left;
}

.action-button:hover {
  background: #f8fafc;
}

.action-button:disabled {
  cursor: not-allowed;
  color: #94a3b8;
  background: #f8fafc;
}

.action-button strong,
.action-button span {
  display: block;
}

.action-button span {
  margin-top: 3px;
  color: #64748b;
  font-size: 12px;
}
</style>

<script setup lang="ts">
defineProps<{
  open: boolean
  title: string
  description: string
  progress: number
  message: string
  logs: string[]
  status: 'idle' | 'running' | 'success' | 'error'
  error?: string
}>()

const emit = defineEmits<{
  close: []
  retry: []
}>()
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="media-tool-backdrop"
      role="presentation"
    >
      <section
        class="media-tool-dialog"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
      >
        <header class="media-tool-header">
          <div>
            <strong>{{ title }}</strong>
            <p>{{ description }}</p>
          </div>
          <button
            type="button"
            class="media-tool-close"
            :disabled="status === 'running'"
            aria-label="关闭"
            @click="emit('close')"
          >
            ×
          </button>
        </header>

        <div
          class="media-tool-progress"
          role="progressbar"
          aria-valuemin="0"
          aria-valuemax="100"
          :aria-valuenow="progress"
        >
          <div
            class="media-tool-progress-fill"
            :style="{ width: `${progress}%` }"
          />
        </div>

        <p class="media-tool-message">
          {{ message || '准备中' }}
        </p>

        <p
          v-if="error"
          class="media-tool-error"
        >
          {{ error }}
        </p>

        <div class="media-tool-log">
          <p
            v-for="(line, index) in logs"
            :key="`${index}-${line}`"
          >
            {{ line }}
          </p>
          <p v-if="!logs.length">
            等待任务开始...
          </p>
        </div>

        <footer class="media-tool-actions">
          <button
            v-if="status === 'error'"
            type="button"
            class="media-tool-retry"
            @click="emit('retry')"
          >
            重试
          </button>
          <button
            type="button"
            class="media-tool-done"
            :disabled="status === 'running'"
            @click="emit('close')"
          >
            {{ status === 'success' ? '完成' : '关闭' }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.media-tool-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px;
  background: rgba(2, 8, 13, 0.72);
  backdrop-filter: blur(10px);
}

.media-tool-dialog {
  width: min(520px, 100%);
  max-height: min(620px, calc(100vh - 36px));
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border: 1px solid rgba(102, 247, 211, 0.24);
  border-radius: var(--bb-radius-lg);
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.11), transparent 36%),
    linear-gradient(180deg, rgba(14, 30, 43, 0.98), rgba(5, 13, 20, 0.98));
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.48);
  color: var(--bb-text);
}

.media-tool-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.media-tool-header strong {
  display: block;
  font-size: 15px;
}

.media-tool-header p,
.media-tool-message,
.media-tool-log p {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.media-tool-close {
  width: 30px;
  min-width: 30px;
  height: 30px;
  padding: 0;
  border-radius: 999px;
}

.media-tool-progress {
  height: 10px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(5, 14, 22, 0.78);
  box-shadow: inset 0 0 0 1px rgba(102, 247, 211, 0.12);
}

.media-tool-progress-fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--bb-primary), #68c9ff);
  transition: width 180ms ease;
}

.media-tool-message {
  color: var(--bb-text);
}

.media-tool-error {
  margin: 0;
  color: var(--bb-danger);
  font-size: 12px;
  overflow-wrap: anywhere;
}

.media-tool-log {
  min-height: 96px;
  max-height: 210px;
  overflow-y: auto;
  display: grid;
  align-content: start;
  gap: 5px;
  padding: 10px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  border-radius: var(--bb-radius-md);
  background: rgba(2, 8, 13, 0.5);
}

.media-tool-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.media-tool-retry,
.media-tool-done {
  min-width: 76px;
}

.media-tool-done:not(:disabled) {
  border-color: rgba(102, 247, 211, 0.55);
  background: var(--bb-primary-soft);
  color: var(--bb-primary-strong);
}
</style>

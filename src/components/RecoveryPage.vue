<script setup lang="ts">
import type { StartupRecoveryStatus } from '@/lib/startup-ipc'

defineOptions({
  name: 'RecoveryPage',
})

defineProps<{
  status: StartupRecoveryStatus
}>()

function retryStartup() {
  window.location.reload()
}

function selectPath(event: FocusEvent) {
  ;(event.currentTarget as HTMLTextAreaElement).select()
}
</script>

<template>
  <main class="recovery-page">
    <section
      class="recovery-panel"
      aria-labelledby="recovery-title"
    >
      <span
        class="recovery-note-mark"
        aria-hidden="true"
      />
      <p class="recovery-eyebrow">
        Banana Box 启动保护
      </p>
      <h1 id="recovery-title">
        先保护本地数据
      </h1>
      <p class="recovery-message">
        {{ status.message }}
      </p>

      <div
        v-if="status.backupPaths.length"
        class="recovery-backups"
      >
        <p>
          已找到的备份路径
        </p>
        <textarea
          v-for="path in status.backupPaths"
          :key="path"
          class="recovery-path"
          :value="path"
          readonly
          rows="2"
          spellcheck="false"
          aria-label="备份文件路径"
          @focus="selectPath"
        />
      </div>

      <p class="recovery-hint">
        路径文本可选中复制。请确认文件仍在原处，再重新检查。
      </p>
      <button
        class="recovery-retry"
        type="button"
        @click="retryStartup"
      >
        重新检查
      </button>
    </section>
  </main>
</template>

<style scoped>
.recovery-page {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: grid;
  place-items: center;
  padding: 28px;
  color: var(--bb-text);
  background:
    radial-gradient(circle at 82% 18%, rgba(255, 209, 102, 0.1), transparent 30%),
    linear-gradient(135deg, rgba(7, 17, 24, 0.98), rgba(13, 24, 35, 0.98) 46%, rgba(4, 12, 19, 0.98)),
    var(--bb-bg);
}

.recovery-panel {
  width: min(100%, 496px);
  padding: 26px;
  overflow: hidden;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: rgba(12, 23, 33, 0.94);
  box-shadow: var(--bb-shadow-dialog);
  animation: recovery-in 200ms ease-out both;
}

.recovery-note-mark {
  display: block;
  width: 38px;
  height: 5px;
  margin-bottom: 18px;
  border-radius: 999px;
  background: var(--bb-favorite);
  box-shadow: 0 0 18px rgba(255, 209, 102, 0.28);
}

.recovery-eyebrow,
.recovery-hint,
.recovery-backups > p {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  line-height: 1.55;
}

.recovery-eyebrow {
  color: var(--bb-favorite);
}

.recovery-panel h1 {
  margin: 8px 0 10px;
  font-size: 22px;
  line-height: 1.28;
  font-weight: 700;
}

.recovery-message {
  margin: 0;
  color: var(--bb-text);
  line-height: 1.65;
}

.recovery-backups {
  display: grid;
  gap: 8px;
  margin-top: 18px;
}

.recovery-path {
  width: 100%;
  min-height: 46px;
  padding: 9px 10px;
  resize: none;
  overflow: auto;
  color: var(--bb-primary-strong);
  font-family: var(--bb-mono);
  font-size: 12px;
  line-height: 1.45;
}

.recovery-hint {
  margin-top: 16px;
}

.recovery-retry {
  margin-top: 18px;
  padding: 7px 12px;
  border-color: rgba(255, 209, 102, 0.38);
  background: rgba(255, 209, 102, 0.12);
  color: #fff0be;
}

.recovery-retry:hover:not(:disabled) {
  border-color: rgba(255, 209, 102, 0.68);
  background: rgba(255, 209, 102, 0.18);
  box-shadow: 0 0 18px rgba(255, 209, 102, 0.16);
}

@keyframes recovery-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
}

@media (prefers-reduced-motion: reduce) {
  .recovery-panel {
    animation: none;
  }
}
</style>

<script setup lang="ts">
import { ref } from 'vue'
import { acknowledgeMigrationSummary, type MigrationSummary } from '@/lib/startup-ipc'

defineOptions({
  name: 'MigrationSummaryDialog',
})

defineProps<{
  summary: MigrationSummary
}>()

const acknowledging = ref(false)
const errorMessage = ref('')
const emit = defineEmits<{
  acknowledged: []
}>()

async function acknowledge() {
  if (acknowledging.value) return

  acknowledging.value = true
  errorMessage.value = ''
  try {
    await acknowledgeMigrationSummary()
    emit('acknowledged')
  } catch {
    errorMessage.value = '暂时无法保存确认状态，请稍后重试。'
  } finally {
    acknowledging.value = false
  }
}
</script>

<template>
  <div class="migration-summary-mask">
    <section
      class="migration-summary-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="migration-summary-title"
    >
      <span
        class="migration-summary-mark"
        aria-hidden="true"
      />
      <p class="migration-summary-eyebrow">
        本地数据已准备就绪
      </p>
      <h2 id="migration-summary-title">
        已完成一次整理
      </h2>
      <p class="migration-summary-intro">
        旧版本数据已按当前格式保存，原始文件也保留了一份备份。
      </p>

      <dl class="migration-summary-stats">
        <div>
          <dt>提示词</dt>
          <dd>{{ summary.promptsMigrated }}</dd>
        </div>
        <div>
          <dt>收藏修正</dt>
          <dd>{{ summary.favoritesDefaulted }}</dd>
        </div>
        <div>
          <dt>排序修正</dt>
          <dd>{{ summary.ordersRebuilt }}</dd>
        </div>
      </dl>

      <p class="migration-summary-path-label">
        备份位置
      </p>
      <code class="migration-summary-path">{{ summary.backupPath }}</code>

      <ul
        v-if="summary.warnings.length"
        class="migration-summary-warnings"
      >
        <li
          v-for="warning in summary.warnings"
          :key="warning"
        >
          {{ warning }}
        </li>
      </ul>

      <p
        v-if="errorMessage"
        class="migration-summary-error"
        role="alert"
      >
        {{ errorMessage }}
      </p>

      <button
        class="migration-summary-confirm"
        type="button"
        :disabled="acknowledging"
        @click="acknowledge"
      >
        {{ acknowledging ? '正在确认...' : '我知道了' }}
      </button>
    </section>
  </div>
</template>

<style scoped>
.migration-summary-mask {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(3, 9, 14, 0.56);
  backdrop-filter: blur(4px);
}

.migration-summary-dialog {
  width: min(100%, 444px);
  max-height: min(100%, 600px);
  padding: 24px;
  overflow-y: auto;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-dialog);
  animation: summary-in 200ms ease-out both;
}

.migration-summary-mark {
  display: block;
  width: 38px;
  height: 5px;
  margin-bottom: 16px;
  border-radius: 999px;
  background: var(--bb-favorite);
  box-shadow: 0 0 18px rgba(255, 209, 102, 0.28);
}

.migration-summary-eyebrow,
.migration-summary-path-label,
.migration-summary-warnings {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 12px;
  line-height: 1.55;
}

.migration-summary-eyebrow {
  color: var(--bb-favorite);
}

.migration-summary-dialog h2 {
  margin: 8px 0 8px;
  font-size: 21px;
  line-height: 1.3;
}

.migration-summary-intro {
  margin: 0;
  color: var(--bb-text);
  line-height: 1.65;
}

.migration-summary-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin: 20px 0;
}

.migration-summary-stats div {
  min-width: 0;
  padding-left: 10px;
  border-left: 2px solid rgba(255, 209, 102, 0.68);
}

.migration-summary-stats dt {
  color: var(--bb-text-muted);
  font-size: 12px;
}

.migration-summary-stats dd {
  margin: 3px 0 0;
  color: var(--bb-primary-strong);
  font-family: var(--bb-mono);
  font-size: 20px;
  line-height: 1;
}

.migration-summary-path-label {
  margin-bottom: 6px;
}

.migration-summary-path {
  display: block;
  padding: 9px 10px;
  overflow-wrap: anywhere;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: rgba(5, 14, 22, 0.82);
  color: var(--bb-primary-strong);
  font-family: var(--bb-mono);
  font-size: 12px;
  line-height: 1.45;
}

.migration-summary-warnings {
  padding-left: 18px;
  margin-top: 14px;
}

.migration-summary-warnings li + li {
  margin-top: 4px;
}

.migration-summary-error {
  margin: 14px 0 0;
  color: var(--bb-danger);
  line-height: 1.55;
}

.migration-summary-confirm {
  width: 100%;
  margin-top: 20px;
  padding: 8px 12px;
  border-color: rgba(255, 209, 102, 0.38);
  background: rgba(255, 209, 102, 0.12);
  color: #fff0be;
}

.migration-summary-confirm:hover:not(:disabled) {
  border-color: rgba(255, 209, 102, 0.68);
  background: rgba(255, 209, 102, 0.18);
  box-shadow: 0 0 18px rgba(255, 209, 102, 0.16);
}

@keyframes summary-in {
  from {
    opacity: 0;
    transform: translateY(8px) scale(0.98);
  }
}

@media (prefers-reduced-motion: reduce) {
  .migration-summary-dialog {
    animation: none;
  }
}
</style>

<script setup lang="ts">
import { onMounted, ref, shallowRef, type Component } from 'vue'
import MigrationSummaryDialog from '@/components/MigrationSummaryDialog.vue'
import RecoveryPage from '@/components/RecoveryPage.vue'
import { getStartupStatus, type StartupStatus } from '@/lib/startup-ipc'

const status = ref<StartupStatus | null>(null)
const readyApp = shallowRef<Component | null>(null)

onMounted(async () => {
  try {
    const startupStatus = await getStartupStatus()
    if (startupStatus.state === 'ready') {
      readyApp.value = (await import('@/components/ReadyApp.vue')).default
    }
    status.value = startupStatus
  } catch {
    status.value = {
      state: 'recovery',
      message: '启动检查未完成。请保留本地文件后重新检查。',
      backupPaths: [],
    }
  }
})

function dismissMigrationSummary() {
  if (status.value?.state !== 'ready') return
  status.value = { ...status.value, migrationSummary: null }
}
</script>

<template>
  <main
    v-if="!status"
    class="startup-loading"
    aria-live="polite"
  >
    <span
      class="startup-loading-mark"
      aria-hidden="true"
    />
    <p>正在检查本地数据...</p>
  </main>
  <template v-else-if="status.state === 'ready'">
    <component
      :is="readyApp"
      v-if="readyApp"
    />
    <MigrationSummaryDialog
      v-if="status.migrationSummary"
      :summary="status.migrationSummary"
      @acknowledged="dismissMigrationSummary"
    />
  </template>
  <RecoveryPage
    v-else
    :status="status"
  />
</template>

<style scoped>
.startup-loading {
  width: 100%;
  height: 100%;
  display: grid;
  align-content: center;
  justify-items: center;
  gap: var(--bb-space-3);
  color: var(--bb-text-muted);
  background:
    linear-gradient(135deg, rgba(7, 17, 24, 0.98), rgba(13, 24, 35, 0.98) 46%, rgba(4, 12, 19, 0.98)),
    var(--bb-bg);
}

.startup-loading p {
  margin: 0;
}

.startup-loading-mark {
  width: 34px;
  height: 7px;
  border-radius: 999px;
  background: var(--bb-favorite);
  box-shadow: 0 0 18px rgba(255, 209, 102, 0.34);
  animation: startup-pulse 1.1s ease-in-out infinite alternate;
}

@keyframes startup-pulse {
  to {
    transform: scaleX(1.28);
    opacity: 0.62;
  }
}

@media (prefers-reduced-motion: reduce) {
  .startup-loading-mark {
    animation: none;
  }
}
</style>

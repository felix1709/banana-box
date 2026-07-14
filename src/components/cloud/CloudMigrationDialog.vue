<script setup lang="ts">
import { computed, watch } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useCloudMigrationStore } from '@/stores/cloudMigration'
import { useWorkspacesStore } from '@/stores/workspaces'

const migration = useCloudMigrationStore()
const auth = useAuthStore()
const workspaces = useWorkspacesStore()

const localTotal = computed(() =>
  migration.comparison.reduce((total: number, row) => total + row.local, 0),
)

const cloudTotal = computed(() =>
  migration.comparison.reduce((total: number, row) => total + row.cloud, 0),
)

const totalPendingUpload = computed(() =>
  migration.comparison.reduce((total: number, row) => total + row.pendingUpload, 0),
)

watch(
  () => [migration.shouldPrompt, Boolean(auth.client), workspaces.activeWorkspaceId] as const,
  ([shouldPrompt, hasClient, activeWorkspaceId]) => {
    if (shouldPrompt && hasClient && activeWorkspaceId) void migration.loadCloudSummary()
  },
  { immediate: true },
)
</script>

<template>
  <div
    v-if="migration.shouldPrompt"
    class="cloud-migration-mask"
  >
    <section
      class="cloud-migration-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="cloud-migration-title"
    >
      <header>
        <p>云端协作</p>
        <h2 id="cloud-migration-title">
          发现本地数据
        </h2>
      </header>
      <p class="cloud-migration-copy">
        当前设备发现本地数据，请对比云端数量后选择是否同步到云端。
      </p>

      <div class="cloud-migration-stats">
        <div
          class="cloud-migration-stat"
          data-migration-stat="local"
        >
          <span>本地</span>
          <strong>{{ localTotal }}</strong>
        </div>
        <div
          class="cloud-migration-stat"
          data-migration-stat="cloud"
        >
          <span>云端</span>
          <strong>{{ migration.cloudSummary.loaded ? cloudTotal : '...' }}</strong>
        </div>
        <div
          class="cloud-migration-stat is-primary"
          data-migration-stat="pending"
        >
          <span>待同步</span>
          <strong>{{ migration.cloudSummary.loaded ? totalPendingUpload : '...' }}</strong>
        </div>
      </div>

      <p class="cloud-migration-hint">
        {{
          migration.cloudSummary.loading
            ? '正在读取云端数据...'
            : migration.cloudSummary.loaded
              ? `请选择是否同步到云端，本次预计同步 ${totalPendingUpload} 项。`
              : '等待读取云端数据...'
        }}
      </p>
      <p
        v-if="migration.error"
        class="cloud-migration-error"
        role="alert"
      >
        {{ migration.error }}
      </p>
      <footer>
        <button
          type="button"
          :disabled="migration.status === 'running'"
          @click="migration.keepLocal()"
        >
          保持本地
        </button>
        <button
          type="button"
          :disabled="migration.status === 'running'"
          @click="migration.decideLater()"
        >
          稍后处理
        </button>
        <button
          class="primary"
          type="button"
          data-action="cloud-migrate-now"
          :disabled="migration.status === 'running'"
          @click="migration.migrateNow()"
        >
          {{ migration.status === 'running' ? '正在同步...' : '同步到云端' }}
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.cloud-migration-mask {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: grid;
  place-items: center;
  padding: 18px;
  background: rgba(2, 8, 13, 0.62);
  backdrop-filter: blur(8px);
}

.cloud-migration-dialog {
  width: min(460px, 100%);
  display: grid;
  gap: 14px;
  max-height: min(620px, calc(100vh - 36px));
  overflow: auto;
  padding: 18px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-dialog);
}

.cloud-migration-dialog header p,
.cloud-migration-dialog header h2,
.cloud-migration-copy,
.cloud-migration-hint {
  margin: 0;
}

.cloud-migration-dialog header p {
  color: var(--bb-primary);
  font-size: 11px;
  font-weight: 700;
}

.cloud-migration-dialog header h2 {
  margin-top: 3px;
  font-size: 18px;
}

.cloud-migration-copy {
  color: var(--bb-text-soft);
  line-height: 1.6;
}

.cloud-migration-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.cloud-migration-stat {
  min-width: 0;
  display: grid;
  gap: 4px;
  padding: 10px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
}

.cloud-migration-stat span {
  color: var(--bb-text-soft);
  font-size: 11px;
}

.cloud-migration-stat strong {
  color: var(--bb-primary-strong);
  font-family: var(--bb-mono);
  font-size: 20px;
  line-height: 1.1;
}

.cloud-migration-stat.is-primary {
  border-color: var(--bb-primary);
  background: var(--bb-primary-soft);
}

.cloud-migration-hint {
  color: var(--bb-text-soft);
  font-size: 12px;
}

.cloud-migration-error {
  margin: 0;
  color: #ffb6c0;
}

.cloud-migration-dialog footer {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 420px) {
  .cloud-migration-stats {
    grid-template-columns: 1fr;
  }
}
</style>

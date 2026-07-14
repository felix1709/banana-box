<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, Cloud, HardDrive, LogOut, Pencil } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const activeWorkspace = computed(() => workspaces.activeWorkspace)
const displayEmail = computed(() => workspaces.profile?.email || auth.user?.email || '')
const editingName = ref(false)
const nameDraft = ref('')
const displayName = computed(() => workspaces.profile?.displayName || displayEmail.value.split('@')[0] || '')
const fallbackWorkspaceName = computed(() => {
  const accountName = displayEmail.value.split('@')[0] || '个人'
  return `${accountName} 的个人空间`
})
const displayWorkspaceName = computed(() => {
  const name = activeWorkspace.value?.name ?? ''
  if (!name) return '尚未选择工作区'
  if (name.includes('?')) return fallbackWorkspaceName.value
  return name
})

async function signOut() {
  await auth.signOut()
  workspaces.clear()
}

function startEditingName() {
  nameDraft.value = displayName.value
  editingName.value = true
}

async function saveDisplayName() {
  if (!auth.client) return
  await workspaces.updateDisplayName(auth.client, nameDraft.value)
  if (!workspaces.error) editingName.value = false
}
</script>

<template>
  <section
    class="workspace-switcher"
    aria-label="当前工作区"
  >
    <div
      v-if="!auth.cloudAvailable"
      class="workspace-state"
    >
      <HardDrive
        :size="14"
        aria-hidden="true"
      />
      <span>本地离线模式</span>
    </div>
    <template v-else-if="auth.user">
      <div class="workspace-state">
        <Cloud
          :size="14"
          aria-hidden="true"
        />
        <span v-if="workspaces.loading">正在创建个人空间</span>
        <span v-else>{{ displayWorkspaceName }}</span>
      </div>
      <p class="workspace-user">
        <span>{{ displayName }}</span>
        <small>{{ displayEmail }}</small>
      </p>
      <div
        v-if="editingName"
        class="workspace-name-editor"
      >
        <input
          v-model="nameDraft"
          data-field="display-name"
          aria-label="协作昵称"
        >
        <button
          type="button"
          data-action="save-display-name"
          :disabled="!nameDraft.trim()"
          @click="saveDisplayName"
        >
          <Check
            :size="14"
            aria-hidden="true"
          />
        </button>
      </div>
      <button
        v-else
        class="workspace-sign-out"
        data-action="edit-display-name"
        type="button"
        @click="startEditingName"
      >
        <Pencil
          :size="14"
          aria-hidden="true"
        />
        <span>编辑昵称</span>
      </button>
      <button
        class="workspace-sign-out"
        data-action="auth-sign-out"
        type="button"
        @click="signOut"
      >
        <LogOut
          :size="14"
          aria-hidden="true"
        />
        <span>退出</span>
      </button>
    </template>
    <div
      v-else
      class="workspace-state"
    >
      <Cloud
        :size="14"
        aria-hidden="true"
      />
      <span>云端未登录</span>
    </div>
  </section>
</template>

<style scoped>
.workspace-switcher {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 8px;
  border: 1px solid rgba(123, 255, 226, 0.14);
  border-radius: var(--bb-radius-md);
  background: rgba(102, 247, 211, 0.06);
  min-width: 0;
}

.workspace-state,
.workspace-sign-out {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.workspace-state {
  color: var(--bb-text);
  font-size: 12px;
  font-weight: 600;
}

.workspace-state span,
.workspace-user,
.workspace-sign-out span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-user {
  margin: 0;
  color: var(--bb-text-muted);
  font-size: 11px;
}

.workspace-user span,
.workspace-user small {
  display: block;
}

.workspace-user span {
  color: var(--bb-text);
  font-size: 12px;
  font-weight: 650;
}

.workspace-name-editor {
  display: grid;
  grid-template-columns: 1fr 32px;
  gap: 6px;
}

.workspace-name-editor input,
.workspace-name-editor button {
  min-height: 30px;
}

.workspace-sign-out {
  width: 100%;
  min-height: 30px;
  justify-content: center;
  padding: 5px 8px;
  border-color: rgba(123, 255, 226, 0.16);
  background: rgba(4, 12, 18, 0.42);
  color: var(--bb-text-muted);
  font-size: 12px;
}
</style>

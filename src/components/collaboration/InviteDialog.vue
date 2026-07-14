<script setup lang="ts">
import { ref } from 'vue'
import type { WorkspaceRole } from '@/types'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useWorkspacesStore } from '@/stores/workspaces'

const props = defineProps<{
  projectId: string | null
}>()

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const members = useMembersStore()
const role = ref<Exclude<WorkspaceRole, 'owner'>>('viewer')
const email = ref('')
const lastInviteUrl = ref('')
const error = ref('')

async function createInvite() {
  if (!auth.client || !auth.user || !workspaces.activeWorkspaceId) return
  error.value = ''
  try {
    const invite = await members.createInvite(auth.client, {
      appOrigin: 'banana-box://invite',
      workspaceId: workspaces.activeWorkspaceId,
      projectId: props.projectId,
      scopeType: props.projectId ? 'project' : 'workspace',
      role: role.value,
      email: email.value.trim() || null,
      userId: auth.user.id,
    })
    lastInviteUrl.value = invite.url
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  }
}
</script>

<template>
  <section class="invite-dialog">
    <label>
      权限
      <select v-model="role">
        <option value="viewer">只读</option>
        <option value="commenter">可评论</option>
        <option value="editor">可编辑</option>
      </select>
    </label>
    <label>
      邮箱
      <input
        v-model="email"
        type="email"
        placeholder="可选"
      >
    </label>
    <button
      type="button"
      data-action="create-invite"
      @click="createInvite"
    >
      生成邀请
    </button>
    <p
      v-if="error"
      class="invite-error"
      role="alert"
    >
      {{ error }}
    </p>
    <textarea
      v-if="lastInviteUrl"
      class="invite-link"
      readonly
      :value="lastInviteUrl"
      aria-label="邀请链接"
    />
    <p
      v-if="lastInviteUrl"
      class="invite-link-text"
    >
      {{ lastInviteUrl }}
    </p>
  </section>
</template>

<style scoped>
.invite-dialog {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: 8px;
}

.invite-dialog label {
  display: grid;
  gap: 4px;
  color: var(--bb-text-soft);
  font-size: 11px;
}

.invite-dialog select,
.invite-dialog input {
  height: 30px;
}

.invite-link {
  width: min(100%, 360px);
  min-height: 34px;
  resize: vertical;
  font-size: 11px;
}

.invite-error {
  margin: 0;
  color: #ffb6c0;
}
</style>

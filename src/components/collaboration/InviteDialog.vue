<script setup lang="ts">
import { ref } from 'vue'
import type { WorkspaceRole } from '@/types'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useWorkspacesStore } from '@/stores/workspaces'

const props = defineProps<{
  projectId: string | null
  canInvite?: boolean
}>()

interface InviteRecipient {
  id: string
  email: string
  displayName: string
}

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const members = useMembersStore()
const role = ref<Exclude<WorkspaceRole, 'owner'>>('viewer')
const query = ref('')
const results = ref<InviteRecipient[]>([])
const searching = ref(false)
const sendingUserId = ref('')
const status = ref('')
const error = ref('')

function accountLabel(email: string) {
  return email.replace('@banana-box.local', '')
}

function ensureInviteReady() {
  if (!auth.client || !auth.user || !workspaces.activeWorkspaceId) {
    error.value = '请先登录云端账号'
    return false
  }
  if (!props.projectId || props.canInvite === false) {
    error.value = '先设为公共项目'
    return false
  }
  return true
}

async function searchUsers() {
  status.value = ''
  error.value = ''
  results.value = []
  if (!ensureInviteReady()) return
  if (!query.value.trim()) {
    error.value = '请输入昵称或账号'
    return
  }
  const client = auth.client
  const user = auth.user
  if (!client || !user) return

  searching.value = true
  try {
    results.value = await members.searchInviteRecipients(client, query.value, user.id)
    if (results.value.length === 0) status.value = '未找到用户'
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    searching.value = false
  }
}

async function addUser(recipient: InviteRecipient) {
  status.value = ''
  error.value = ''
  if (!ensureInviteReady() || !workspaces.activeWorkspaceId || !props.projectId) return
  const client = auth.client
  const user = auth.user
  if (!client || !user) return

  sendingUserId.value = recipient.id
  try {
    await members.createProjectUserInvite(client, {
      workspaceId: workspaces.activeWorkspaceId,
      projectId: props.projectId,
      role: role.value,
      recipient,
      userId: user.id,
    })
    status.value = `已发送给 ${recipient.displayName || accountLabel(recipient.email)}`
    results.value = results.value.filter((item) => item.id !== recipient.id)
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    sendingUserId.value = ''
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
    <label class="invite-search-field">
      用户
      <input
        v-model="query"
        data-field="invite-search"
        placeholder="昵称或 000002"
        @keydown.enter.prevent="searchUsers"
      >
    </label>
    <button
      type="button"
      data-action="search-invite-users"
      :disabled="searching"
      @click="searchUsers"
    >
      {{ searching ? '搜索中' : '搜索' }}
    </button>

    <div
      v-if="results.length > 0"
      class="invite-results"
    >
      <article
        v-for="recipient in results"
        :key="recipient.id"
        class="invite-result"
      >
        <span>
          <strong>{{ recipient.displayName || accountLabel(recipient.email) }}</strong>
          <small>{{ accountLabel(recipient.email) }}</small>
        </span>
        <button
          type="button"
          data-action="add-invite-user"
          :data-user-id="recipient.id"
          :disabled="sendingUserId === recipient.id"
          @click="addUser(recipient)"
        >
          {{ sendingUserId === recipient.id ? '发送中' : '添加' }}
        </button>
      </article>
    </div>

    <p
      v-if="status"
      class="invite-status"
    >
      {{ status }}
    </p>
    <p
      v-if="error"
      class="invite-error"
      role="alert"
    >
      {{ error }}
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

.invite-search-field {
  min-width: 170px;
  flex: 1;
}

.invite-results {
  display: grid;
  width: 100%;
  gap: 6px;
}

.invite-result {
  display: flex;
  min-width: 0;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 7px;
  border: 1px solid rgba(123, 255, 226, 0.14);
  border-radius: var(--bb-radius-sm);
  background: rgba(123, 255, 226, 0.06);
}

.invite-result span {
  display: grid;
  min-width: 0;
  gap: 2px;
}

.invite-result strong,
.invite-result small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.invite-result strong {
  color: var(--bb-text);
  font-size: 12px;
}

.invite-result small {
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.invite-result button {
  flex: 0 0 auto;
  min-height: 26px;
  padding: 0 8px;
}

.invite-status,
.invite-error {
  width: 100%;
  margin: 0;
  font-size: 12px;
}

.invite-status {
  color: var(--bb-primary-strong);
}

.invite-error {
  color: #ffb6c0;
}
</style>

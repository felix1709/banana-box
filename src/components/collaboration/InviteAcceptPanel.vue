<script setup lang="ts">
import { ref } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'

const auth = useAuthStore()
const members = useMembersStore()
const token = ref('')
const status = ref('')
const error = ref('')

function normalizeInviteToken(value: string) {
  const trimmed = value.trim()
  try {
    return new URL(trimmed).searchParams.get('token') ?? trimmed
  } catch {
    return trimmed
  }
}

async function acceptInvite() {
  if (!auth.client || !auth.user || !token.value.trim()) return
  status.value = ''
  error.value = ''
  try {
    await members.acceptInvite(auth.client, normalizeInviteToken(token.value))
    token.value = ''
    status.value = '已加入'
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  }
}
</script>

<template>
  <section
    v-if="auth.user"
    class="invite-accept-panel"
  >
    <input
      v-model="token"
      placeholder="粘贴邀请 token"
      aria-label="邀请 token"
    >
    <button
      type="button"
      data-action="accept-invite"
      :disabled="!token.trim()"
      @click="acceptInvite"
    >
      加入
    </button>
    <p
      v-if="status"
      class="invite-accept-status"
    >
      {{ status }}
    </p>
    <p
      v-if="error"
      class="invite-accept-error"
      role="alert"
    >
      {{ error }}
    </p>
  </section>
</template>

<style scoped>
.invite-accept-panel {
  display: grid;
  gap: 6px;
}

.invite-accept-panel input {
  width: 100%;
  min-width: 0;
  height: 28px;
  padding: 0 8px;
  font-size: 12px;
}

.invite-accept-panel button {
  min-height: 28px;
  padding: 0 8px;
  font-size: 12px;
}

.invite-accept-status,
.invite-accept-error {
  margin: 0;
  font-size: 11px;
}

.invite-accept-status {
  color: var(--bb-primary-strong);
}

.invite-accept-error {
  color: #ffb6c0;
}
</style>

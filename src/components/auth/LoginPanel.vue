<script setup lang="ts">
import { ref } from 'vue'
import { LogIn, MailCheck } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const account = ref('')
const password = ref('')

async function submit() {
  await auth.signIn(account.value.trim(), password.value)
  password.value = ''
}
</script>

<template>
  <section
    class="login-panel"
    aria-label="账号登录"
  >
    <p class="login-title">
      账号登录
    </p>
    <input
      v-model="account"
      class="login-input"
      data-field="auth-account"
      type="text"
      autocomplete="username"
      placeholder="账号或邮箱"
    >
    <input
      v-model="password"
      class="login-input"
      data-field="auth-password"
      type="password"
      autocomplete="current-password"
      placeholder="密码"
      @keydown.enter="submit"
    >
    <button
      class="login-submit"
      data-action="auth-submit"
      type="button"
      :disabled="auth.loading || !account || password.length < 6"
      @click="submit"
    >
      <LogIn
        :size="14"
        aria-hidden="true"
      />
      <span>登录</span>
    </button>
    <div
      v-if="auth.error"
      class="login-error"
    >
      <p>{{ auth.error }}</p>
      <button
        v-if="auth.emailAwaitingConfirmation"
        class="login-resend"
        data-action="auth-resend-confirmation"
        type="button"
        :disabled="auth.loading"
        @click="auth.resendConfirmation()"
      >
        <MailCheck
          :size="14"
          aria-hidden="true"
        />
        <span>重新发送验证邮件</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.login-panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  border: 1px solid rgba(123, 255, 226, 0.14);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.42);
}

.login-title,
.login-error p {
  margin: 0;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.login-title {
  color: var(--bb-text);
  font-size: 12px;
  font-weight: 600;
}

.login-input {
  width: 100%;
  min-height: 30px;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid rgba(123, 255, 226, 0.18);
  border-radius: var(--bb-radius-sm);
  background: rgba(4, 12, 18, 0.72);
  color: var(--bb-text);
  font: inherit;
  font-size: 12px;
}

.login-input:focus-visible {
  outline: none;
  box-shadow: var(--bb-focus);
}

.login-submit,
.login-resend {
  width: 100%;
  min-height: 32px;
  justify-content: center;
  padding: 6px 8px;
  font-size: 12px;
}

.login-submit,
.login-resend {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  border-color: rgba(102, 247, 211, 0.34);
  background: var(--bb-primary-soft);
  color: var(--bb-text);
}

.login-submit:disabled,
.login-resend:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.login-error {
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: #ffb4a8;
  font-size: 11px;
  line-height: 1.45;
  white-space: normal;
}

.login-resend {
  min-height: 30px;
}
</style>

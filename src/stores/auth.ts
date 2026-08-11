import { defineStore } from 'pinia'
import type { Session, SupabaseClient, User } from '@supabase/supabase-js'
import { getSupabaseClient } from '@/lib/supabaseClient'

const TEST_ACCOUNT_DOMAIN = 'banana-box.local'
const LEGACY_ADMIN_EMAIL = `000001@${TEST_ACCOUNT_DOMAIN}`
const EMAIL_NOT_CONFIRMED_MESSAGE = '邮箱还没有验证，请先打开邮箱里的 Supabase 验证邮件。也可以点击下方按钮重新发送验证邮件。'

function normalizeLoginAccount(account: string) {
  const normalized = account.trim()
  if (/^\d{6}$/.test(normalized)) return `${normalized}@${TEST_ACCOUNT_DOMAIN}`
  return normalized
}

function translateAuthError(message: string) {
  const lower = message.toLowerCase()
  if (lower.includes('email not confirmed')) return EMAIL_NOT_CONFIRMED_MESSAGE
  if (lower.includes('invalid login credentials')) return '账号或密码不正确，请检查后再试。'
  if (message === 'CLOUD_NOT_CONFIGURED') return '还没有配置云端，请先在设置里填写 Supabase URL 和 anon key。'
  return message
}

export const useAuthStore = defineStore('auth', {
  state: () => ({
    client: null as SupabaseClient | null,
    session: null as Session | null,
    user: null as User | null,
    cloudAvailable: false,
    loading: false,
    error: '',
    emailAwaitingConfirmation: '',
    unsubscribeAuth: null as null | (() => void),
  }),
  getters: {
    isCloudAdmin(state) {
      return (
        state.user?.app_metadata?.cloud_admin === true
        || state.user?.email?.toLowerCase() === LEGACY_ADMIN_EMAIL
      )
    },
  },
  actions: {
    async initialize() {
      this.loading = true
      this.error = ''
      this.client = await getSupabaseClient()
      this.cloudAvailable = Boolean(this.client)
      if (!this.client) {
        this.session = null
        this.user = null
        this.emailAwaitingConfirmation = ''
        this.loading = false
        return
      }

      const { data, error } = await this.client.auth.getSession()
      if (error) this.error = translateAuthError(error.message)
      this.session = data.session
      this.user = data.session?.user ?? null
      if (this.user) await this.refreshUser()

      this.unsubscribeAuth?.()
      const { data: listener } = this.client.auth.onAuthStateChange((_event, session) => {
        this.session = session
        this.user = session?.user ?? null
        if (this.user) void this.refreshUser()
      })
      this.unsubscribeAuth = () => listener.subscription.unsubscribe()
      this.loading = false
    },
    async refreshUser() {
      if (!this.client) return
      const { data, error } = await this.client.auth.getUser()
      if (error) {
        this.error = translateAuthError(error.message)
        return
      }
      if (data.user) this.user = data.user
    },
    async signIn(account: string, password: string) {
      if (!this.client) await this.initialize()
      if (!this.client) {
        this.error = translateAuthError('CLOUD_NOT_CONFIGURED')
        return
      }
      this.loading = true
      this.error = ''
      this.emailAwaitingConfirmation = ''
      const email = normalizeLoginAccount(account)
      const { data, error } = await this.client.auth.signInWithPassword({ email, password })
      if (error) {
        this.error = translateAuthError(error.message)
        if (error.message.toLowerCase().includes('email not confirmed')) {
          this.emailAwaitingConfirmation = email
        }
        this.session = null
        this.user = null
      } else {
        this.session = data.session
        this.user = data.session?.user ?? null
        if (this.user) await this.refreshUser()
      }
      this.loading = false
    },
    async signUp(email: string, password: string) {
      if (!this.client) await this.initialize()
      if (!this.client) {
        this.error = translateAuthError('CLOUD_NOT_CONFIGURED')
        return
      }
      this.loading = true
      this.error = ''
      this.emailAwaitingConfirmation = ''
      const normalizedEmail = normalizeLoginAccount(email)
      const { data, error } = await this.client.auth.signUp({ email: normalizedEmail, password })
      if (error) this.error = translateAuthError(error.message)
      this.session = data.session
      this.user = data.user ?? data.session?.user ?? null
      if (!error && data.user && !data.session) {
        this.emailAwaitingConfirmation = normalizedEmail
        this.error = '注册成功，请打开邮箱里的 Supabase 验证邮件后再登录。'
      }
      if (this.user) await this.refreshUser()
      this.loading = false
    },
    async resendConfirmation() {
      if (!this.emailAwaitingConfirmation) return
      if (!this.client) await this.initialize()
      if (!this.client) {
        this.error = translateAuthError('CLOUD_NOT_CONFIGURED')
        return
      }
      this.loading = true
      const { error } = await this.client.auth.resend({
        type: 'signup',
        email: this.emailAwaitingConfirmation,
      })
      this.error = error
        ? translateAuthError(error.message)
        : '验证邮件已重新发送，请检查收件箱和垃圾邮件。'
      this.loading = false
    },
    async signOut() {
      if (this.client) await this.client.auth.signOut()
      this.session = null
      this.user = null
      this.emailAwaitingConfirmation = ''
    },
    dispose() {
      this.unsubscribeAuth?.()
      this.unsubscribeAuth = null
    },
  },
})

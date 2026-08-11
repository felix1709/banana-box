import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuthStore } from '@/stores/auth'
import { getSupabaseClient } from '@/lib/supabaseClient'

const authApi = {
  getSession: vi.fn(),
  getUser: vi.fn(),
  resend: vi.fn(),
  signInWithPassword: vi.fn(),
  signUp: vi.fn(),
  signOut: vi.fn(),
  onAuthStateChange: vi.fn(),
}

vi.mock('@/lib/supabaseClient', () => ({
  getSupabaseClient: vi.fn(),
}))

describe('auth store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    authApi.onAuthStateChange.mockReturnValue({
      data: { subscription: { unsubscribe: vi.fn() } },
    })
    authApi.getUser.mockResolvedValue({ data: { user: null }, error: null })
    authApi.resend.mockResolvedValue({ data: {}, error: null })
  })

  it('stays local-only when no Supabase client is available', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue(null)
    const store = useAuthStore()

    await store.initialize()

    expect(store.cloudAvailable).toBe(false)
    expect(store.user).toBeNull()
  })

  it('restores an existing session', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({
      data: { session: { user: { id: 'user-1', email: 'a@example.com' } } },
      error: null,
    })
    const store = useAuthStore()

    await store.initialize()

    expect(store.cloudAvailable).toBe(true)
    expect(store.user?.id).toBe('user-1')
  })

  it('refreshes the signed-in user from the server so corrected emails replace stale session emails', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({
      data: { session: { user: { id: 'user-1', email: 'wrong@example.com' } } },
      error: null,
    })
    authApi.getUser.mockResolvedValue({
      data: { user: { id: 'user-1', email: 'correct@example.com' } },
      error: null,
    })
    const store = useAuthStore()

    await store.initialize()

    expect(authApi.getUser).toHaveBeenCalled()
    expect(store.user?.email).toBe('correct@example.com')
  })

  it('signs in with email and password', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: { user: { id: 'user-2', email: 'b@example.com' } } },
      error: null,
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('b@example.com', 'password123')

    expect(authApi.signInWithPassword).toHaveBeenCalledWith({
      email: 'b@example.com',
      password: 'password123',
    })
    expect(store.user?.id).toBe('user-2')
  })

  it('maps six-digit test accounts to the local Banana Box email domain', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: { user: { id: 'user-001', email: '000001@banana-box.local' } } },
      error: null,
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('000001', '123456')

    expect(authApi.signInWithPassword).toHaveBeenCalledWith({
      email: '000001@banana-box.local',
      password: '123456',
    })
  })

  it('derives cloud administrator access from the 000001 account or cloud_admin app metadata claim', () => {
    const store = useAuthStore()

    store.user = { id: 'legacy', email: '000001@banana-box.local' } as never
    expect(store.isCloudAdmin).toBe(true)

    store.user = { id: 'member', email: '000002@banana-box.local' } as never
    expect(store.isCloudAdmin).toBe(false)

    store.user = {
      id: 'admin',
      email: 'admin@example.com',
      app_metadata: { cloud_admin: true },
    } as never
    expect(store.isCloudAdmin).toBe(true)
  })

  it('records sign-in errors without throwing', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: null },
      error: { message: 'Invalid login credentials' },
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('bad@example.com', 'bad')

    expect(store.error).toBe('账号或密码不正确，请检查后再试。')
    expect(store.user).toBeNull()
  })

  it('turns unconfirmed email errors into an actionable confirmation message', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: null },
      error: { message: 'Email not confirmed' },
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('new@example.com', 'password123')

    expect(store.error).toBe('邮箱还没有验证，请先打开邮箱里的 Supabase 验证邮件。也可以点击下方按钮重新发送验证邮件。')
    expect(store.emailAwaitingConfirmation).toBe('new@example.com')
  })

  it('resends the confirmation email for the pending address', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    const store = useAuthStore()
    await store.initialize()
    store.emailAwaitingConfirmation = 'new@example.com'

    await store.resendConfirmation()

    expect(authApi.resend).toHaveBeenCalledWith({
      type: 'signup',
      email: 'new@example.com',
    })
    expect(store.error).toBe('验证邮件已重新发送，请检查收件箱和垃圾邮件。')
  })
})

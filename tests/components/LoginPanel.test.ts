import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import LoginPanel from '@/components/auth/LoginPanel.vue'
import { useAuthStore } from '@/stores/auth'

describe('LoginPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('signs in with a test account id and password', async () => {
    const auth = useAuthStore()
    auth.signIn = vi.fn()
    const wrapper = mount(LoginPanel)

    await wrapper.find('[data-field="auth-account"]').setValue('000001')
    await wrapper.find('[data-field="auth-password"]').setValue('123456')
    await wrapper.find('[data-action="auth-submit"]').trigger('click')

    expect(auth.signIn).toHaveBeenCalledWith('000001', '123456')
  })

  it('still accepts a full email address for normal login', async () => {
    const auth = useAuthStore()
    auth.signIn = vi.fn()
    const wrapper = mount(LoginPanel)

    await wrapper.find('[data-field="auth-account"]').setValue('a@example.com')
    await wrapper.find('[data-field="auth-password"]').setValue('password123')
    await wrapper.find('[data-action="auth-submit"]').trigger('click')

    expect(auth.signIn).toHaveBeenCalledWith('a@example.com', 'password123')
  })

  it('does not show self-registration controls for the first test batch', () => {
    const wrapper = mount(LoginPanel)

    expect(wrapper.find('[data-action="auth-mode"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('注册账号')
  })

  it('shows a resend confirmation button when email confirmation is pending', async () => {
    const auth = useAuthStore()
    auth.error = '邮箱还没有验证，请先打开邮箱里的 Supabase 验证邮件。也可以点击下方按钮重新发送验证邮件。'
    auth.emailAwaitingConfirmation = 'new@example.com'
    auth.resendConfirmation = vi.fn()
    const wrapper = mount(LoginPanel)

    expect(wrapper.text()).toContain('重新发送验证邮件')
    await wrapper.find('[data-action="auth-resend-confirmation"]').trigger('click')

    expect(auth.resendConfirmation).toHaveBeenCalled()
  })
})

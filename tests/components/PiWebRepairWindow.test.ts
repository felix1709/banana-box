import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PiWebRepairWindow from '@/components/piweb/PiWebRepairWindow.vue'

const api = vi.hoisted(() => ({
  getPiWebConfigStatus: vi.fn(),
  repairPiWebConfig: vi.fn(),
}))

vi.mock('@/lib/piWebIpc', () => api)

describe('PiWebRepairWindow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    api.getPiWebConfigStatus.mockResolvedValue({
      agentDir: 'C:\\Users\\tester\\.pi\\agent',
      settingsExists: false,
      modelsExists: false,
      authExists: false,
      defaultProvider: '雷火',
      defaultModel: 'glm-5.2',
      providerConfigured: false,
      authConfigured: false,
      needsRepair: true,
      message: 'PI-Web 配置需要修复',
      detail: '填写 API Key 后可以一键修复。',
    })
    api.repairPiWebConfig.mockResolvedValue({
      changed: true,
      message: 'PI-Web 配置已修复',
      detail: '请重新启动 PI-Web。',
      status: {
        agentDir: 'C:\\Users\\tester\\.pi\\agent',
        settingsExists: true,
        modelsExists: true,
        authExists: true,
        defaultProvider: '雷火',
        defaultModel: 'glm-5.2',
        providerConfigured: true,
        authConfigured: true,
        needsRepair: false,
        message: 'PI-Web 配置已就绪',
        detail: '当前配置可以使用。',
      },
    })
  })

  it('renders the PI-Web config repair controls in an isolated scrollable window', async () => {
    const wrapper = mount(PiWebRepairWindow)
    await vi.dynamicImportSettled()

    expect(api.getPiWebConfigStatus).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('配置修复')
    expect(wrapper.text()).toContain('PI-Web 配置需要修复')
    expect(wrapper.text()).toContain('雷火 / glm-5.2')
    expect(wrapper.get('[data-field="pi-web-api-key"]').attributes('type')).toBe('password')
    expect(wrapper.find('.pi-web-repair-body').exists()).toBe(true)
  })

  it('submits the user API key without echoing it back into the page', async () => {
    const wrapper = mount(PiWebRepairWindow)
    await vi.dynamicImportSettled()

    await wrapper.get('[data-field="pi-web-api-key"]').setValue('sk-user-secret')
    await wrapper.get('[data-action="repair-pi-web-config"]').trigger('click')

    expect(api.repairPiWebConfig).toHaveBeenCalledWith('sk-user-secret')
    expect(wrapper.text()).toContain('PI-Web 配置已修复')
    expect(wrapper.text()).not.toContain('sk-user-secret')
    expect((wrapper.get('[data-field="pi-web-api-key"]').element as HTMLInputElement).value).toBe('')
  })
})

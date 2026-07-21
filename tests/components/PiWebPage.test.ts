import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PiWebPage from '@/components/piweb/PiWebPage.vue'

const status = {
  state: 'stopped',
  url: 'http://127.0.0.1:30141',
  port: 30141,
  message: 'PI-Web 未启动',
  detail: '',
  missingDependency: '',
  installLinks: [],
  canStart: true,
  canOpen: false,
  canStop: false,
} as const

const api = vi.hoisted(() => ({
  getPiWebStatus: vi.fn(),
  startPiWeb: vi.fn(),
  stopPiWeb: vi.fn(),
  openPiWeb: vi.fn(),
  getPiWebChatHealth: vi.fn(),
  repairPiWebModelCompatibility: vi.fn(),
}))

vi.mock('@/lib/piWebIpc', () => api)

describe('PiWebPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    api.getPiWebStatus.mockResolvedValue({ ...status })
    api.startPiWeb.mockResolvedValue({
      ...status,
      state: 'running',
      message: 'PI-Web 正在运行',
      canStart: false,
      canOpen: true,
      canStop: true,
    })
    api.stopPiWeb.mockResolvedValue({ ...status })
    api.openPiWeb.mockResolvedValue({
      ...status,
      state: 'running',
      message: 'PI-Web 正在运行',
      canStart: false,
      canOpen: true,
      canStop: true,
    })
    api.getPiWebChatHealth.mockResolvedValue({
      state: 'ok',
      message: '对话检测通过',
      detail: 'PI-Web 可以正常收到模型回复。',
      provider: 'OpenAI',
      modelId: 'gpt-5',
    })
    api.repairPiWebModelCompatibility.mockResolvedValue({
      changed: true,
      message: '已写入兼容配置',
      detail: '请停止并重新启动 PI-Web。',
    })
  })

  it('shows status and starts PI-Web from the main action', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('PI-Web')
    expect(wrapper.text()).toContain('http://127.0.0.1:30141')

    await wrapper.get('[data-action="start-pi-web"]').trigger('click')

    expect(api.startPiWeb).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('PI-Web 正在运行')
  })

  it('shows missing dependency links', async () => {
    api.getPiWebStatus.mockResolvedValue({
      ...status,
      state: 'missingRuntime',
      message: '缺少 Node.js',
      missingDependency: 'Node.js',
      canStart: false,
      installLinks: [{ label: '下载 Node.js', url: 'https://nodejs.org/' }],
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('缺少 Node.js')
    expect(wrapper.get('[data-install-link="Node.js"]').attributes('href')).toBe('https://nodejs.org/')
  })

  it('shows chat health errors from the diagnostic action', async () => {
    api.getPiWebStatus.mockResolvedValue({
      ...status,
      state: 'running',
      message: 'PI-Web 正在运行',
      canStart: false,
      canOpen: true,
      canStop: true,
    })
    api.getPiWebChatHealth.mockResolvedValue({
      state: 'error',
      message: '模型接口不兼容',
      detail: "400: developer is not one of ['system', 'assistant', 'user']",
      provider: '雷火',
      modelId: 'glm-5.2',
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    await wrapper.get('[data-action="check-pi-web-chat"]').trigger('click')

    expect(api.getPiWebChatHealth).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('模型接口不兼容')
    expect(wrapper.text()).toContain('glm-5.2')
  })

  it('repairs developer-role model compatibility from the health card', async () => {
    api.getPiWebStatus.mockResolvedValue({
      ...status,
      state: 'running',
      message: 'PI-Web 正在运行',
      canStart: false,
      canOpen: true,
      canStop: true,
    })
    api.getPiWebChatHealth.mockResolvedValue({
      state: 'error',
      message: '模型接口不兼容',
      detail: "400: developer is not one of ['system', 'assistant', 'user']",
      provider: '雷火',
      modelId: 'glm-5.2',
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()
    await wrapper.get('[data-action="check-pi-web-chat"]').trigger('click')
    await wrapper.get('[data-action="repair-pi-web-model"]').trigger('click')

    expect(api.repairPiWebModelCompatibility).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('已写入兼容配置')
  })
})

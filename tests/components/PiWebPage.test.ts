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
  getPiWebConfigStatus: vi.fn(),
  repairPiWebConfig: vi.fn(),
  openPiWebRepairWindow: vi.fn(),
  getPiWebRuntimeStatus: vi.fn(),
  installPiWebRuntime: vi.fn(),
  setPiWebRuntimeDirectory: vi.fn(),
  cleanPiWebRuntime: vi.fn(),
}))

vi.mock('@/lib/piWebIpc', () => api)

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => vi.fn()),
}))

const dialog = vi.hoisted(() => ({ open: vi.fn() }))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: dialog.open,
}))

const runtimeStatus = {
  source: 'bundled',
  state: 'updateAvailable',
  installed: true,
  version: '0.7.16',
  latestVersion: '0.9.0',
  installDir: '',
  nodePath: 'C:\\node.exe',
  npmPath: 'C:\\npm-cli.js',
  canInstall: false,
  canUpdate: true,
  canClean: false,
  message: '当前使用安装包内置版本 0.7.16',
  detail: '检测到最新版本 0.9.0。',
} as const

describe('PiWebPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    api.getPiWebStatus.mockResolvedValue({ ...status })
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
      detail: '请填写自己的 API Key 后点击一键修复。',
    })
    api.repairPiWebConfig.mockResolvedValue({
      changed: true,
      message: 'PI-Web 配置已修复',
      detail: '请重新启动 PI-Web 后检测对话。',
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
        detail: '当前用户已配置雷火 / glm-5.2。',
      },
    })
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
    api.openPiWebRepairWindow.mockResolvedValue(undefined)
    api.getPiWebRuntimeStatus.mockResolvedValue({ ...runtimeStatus })
    api.installPiWebRuntime.mockResolvedValue({
      ...runtimeStatus,
      source: 'managed',
      state: 'ready',
      version: '0.9.0',
      latestVersion: '0.9.0',
      canUpdate: false,
      canClean: true,
      message: 'PI-WEB 0.9.0 已就绪',
    })
    api.setPiWebRuntimeDirectory.mockResolvedValue({
      ...runtimeStatus,
      source: 'custom',
      state: 'ready',
      canUpdate: false,
      message: '正在使用手动指定的 PI-WEB',
    })
    api.cleanPiWebRuntime.mockResolvedValue({
      ...runtimeStatus,
      source: 'none',
      state: 'notInstalled',
      installed: false,
      version: '',
      canUpdate: false,
      canInstall: true,
      canClean: false,
      message: '尚未安装 PI-WEB',
    })
  })

  it('shows status and starts PI-Web from the main action', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('PI-Web')
    expect(wrapper.text()).toContain('http://127.0.0.1:30141')
    expect(wrapper.text()).toContain('未启动')
    expect(wrapper.text()).not.toContain('Local agent console')

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
  it('renders PI-Web one-click config controls directly on the main page', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(api.getPiWebConfigStatus).toHaveBeenCalledOnce()
    expect(wrapper.find('[data-field="pi-web-base-url"]').exists()).toBe(true)
    expect(wrapper.get<HTMLInputElement>('[data-field="pi-web-base-url"]').element.value).toBe(
      'https://ai.leihuo.netease.com/v1',
    )
    expect(wrapper.find('[data-field="pi-web-api-key"]').exists()).toBe(true)
    expect(wrapper.find('[data-action="repair-pi-web-config"]').exists()).toBe(true)
    expect(wrapper.find('[data-action="open-pi-web-repair"]').exists()).toBe(false)
  })

  it('writes PI-Web URL and key from the main page without opening a secondary window', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    await wrapper.get('[data-field="pi-web-base-url"]').setValue('https://ai.leihuo.netease.com/v1/')
    await wrapper.get('[data-field="pi-web-api-key"]').setValue('sk-test-main-page')
    await wrapper.get('[data-action="repair-pi-web-config"]').trigger('click')

    expect(api.repairPiWebConfig).toHaveBeenCalledWith(
      'sk-test-main-page',
      'https://ai.leihuo.netease.com/v1',
    )
    expect(api.openPiWebRepairWindow).not.toHaveBeenCalled()
    expect(wrapper.get<HTMLInputElement>('[data-field="pi-web-api-key"]').element.value).toBe('')
  })

  it('offers one-click download when no PI-WEB is installed yet', async () => {
    api.getPiWebRuntimeStatus.mockResolvedValue({
      ...runtimeStatus,
      source: 'none',
      state: 'notInstalled',
      installed: false,
      version: '',
      canUpdate: false,
      canInstall: true,
      message: '尚未安装 PI-WEB',
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('尚未安装 PI-WEB')
    expect(wrapper.text()).toContain('未安装')

    await wrapper.get('[data-action="install-pi-web-runtime"]').trigger('click')
    await vi.dynamicImportSettled()

    expect(api.installPiWebRuntime).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('PI-WEB 已就绪，点击「启动 PI-Web」即可使用。')
    expect(wrapper.text()).toContain('PI-WEB 0.9.0 已就绪')
  })

  it('offers an update when a newer PI-WEB exists', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('当前使用安装包内置版本 0.7.16')
    expect(wrapper.text()).toContain('最新 0.9.0')
    expect(wrapper.get('[data-action="install-pi-web-runtime"]').text()).toContain('更新 PI-WEB')
  })

  it('lets users point at an existing PI-WEB folder', async () => {
    dialog.open.mockResolvedValue('D:\\pi-web')

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()
    await wrapper.get('[data-action="select-pi-web-directory"]').trigger('click')
    await vi.dynamicImportSettled()

    expect(api.setPiWebRuntimeDirectory).toHaveBeenCalledWith('D:\\pi-web')
    expect(wrapper.text()).toContain('已切换到手动指定的 PI-WEB 目录。')
  })

  it('cleans only the downloaded copy after the user confirms', async () => {
    api.getPiWebRuntimeStatus.mockResolvedValue({
      ...runtimeStatus,
      source: 'managed',
      state: 'ready',
      version: '0.9.0',
      latestVersion: '0.9.0',
      canUpdate: false,
      canClean: true,
    })
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true)

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()
    await wrapper.get('[data-action="clean-pi-web-runtime"]').trigger('click')
    await vi.dynamicImportSettled()

    expect(confirm).toHaveBeenCalledOnce()
    expect(api.cleanPiWebRuntime).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('已清理一键下载的 PI-WEB。')
    confirm.mockRestore()
  })

  it('keeps the cleanup action away from a manually selected folder', async () => {
    api.getPiWebRuntimeStatus.mockResolvedValue({
      ...runtimeStatus,
      source: 'custom',
      state: 'ready',
      installDir: 'D:\\pi-web',
      canUpdate: false,
      canClean: false,
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.find('[data-action="clean-pi-web-runtime"]').exists()).toBe(false)
    expect(wrapper.find('[data-action="reset-pi-web-directory"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('D:\\pi-web')
  })
})

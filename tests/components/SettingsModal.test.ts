import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsModal from '@/components/SettingsModal.vue'
import { checkApiConnection } from '@/lib/ipc'
import { checkAppUpdate, installAppUpdate } from '@/lib/updater'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn().mockResolvedValue(undefined),
  disable: vi.fn().mockResolvedValue(undefined),
  isEnabled: vi.fn().mockResolvedValue(false),
}))

vi.mock('@/lib/ipc', () => ({
  exportLibrary: vi.fn().mockResolvedValue(undefined),
  importLibrary: vi.fn().mockResolvedValue(null),
  readImportDir: vi.fn().mockResolvedValue([]),
  downloadImage: vi.fn(),
  checkApiConnection: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@/lib/updater', () => ({
  checkAppUpdate: vi.fn(),
  installAppUpdate: vi.fn(),
}))

async function openSettingsTab(wrapper: ReturnType<typeof mount>, index: number) {
  await wrapper.findAll('.settings-tab')[index].trigger('click')
}

describe('SettingsModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(isEnabled).mockResolvedValue(false)
  })

  it('groups settings into feature, API, hotkey, and import/export pages', async () => {
    const wrapper = mount(SettingsModal)

    const tabs = wrapper.findAll('.settings-tab').map((tab) => tab.text())

    expect(tabs).toEqual(['功能设置', 'API设置', '快捷键', '导入导出'])
    expect(wrapper.find('.autostart-toggle').exists()).toBe(true)

    await openSettingsTab(wrapper, 1)

    expect(wrapper.find('.api-panel').exists()).toBe(true)
    expect(wrapper.find('.autostart-toggle').exists()).toBe(false)
  })

  it('loads and toggles system autostart from feature settings', async () => {
    vi.mocked(isEnabled).mockResolvedValue(true)
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const toggle = wrapper.find('.autostart-toggle')
    expect((toggle.element as HTMLInputElement).checked).toBe(true)

    await toggle.setValue(false)

    expect(disable).toHaveBeenCalled()
    expect(enable).not.toHaveBeenCalled()
  })

  it('checks for updates and installs a newer version inside the app', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue({
      currentVersion: '0.1.2',
      latestVersion: '0.1.3',
      updateAvailable: true,
    })
    vi.mocked(installAppUpdate).mockResolvedValue(undefined)
    const wrapper = mount(SettingsModal)

    await wrapper.find('.version-check-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(checkAppUpdate).toHaveBeenCalled()
    expect(wrapper.find('.version-status').text()).toContain('0.1.3')

    await wrapper.find('.install-update-button').trigger('click')

    expect(installAppUpdate).toHaveBeenCalled()
  })

  it('shows an up-to-date message when no newer release exists', async () => {
    vi.mocked(checkAppUpdate).mockResolvedValue({
      currentVersion: '0.1.2',
      latestVersion: '0.1.2',
      updateAvailable: false,
    })
    const wrapper = mount(SettingsModal)

    await wrapper.find('.version-check-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.install-update-button').exists()).toBe(false)
    expect(wrapper.find('.version-status').text()).toContain('0.1.2')
  })

  it('shows compact reverse API settings with safe defaults', async () => {
    const wrapper = mount(SettingsModal)
    await openSettingsTab(wrapper, 1)

    expect((wrapper.find('.api-base-url-input').element as HTMLInputElement).value).toBe(
      'https://ai.leihuo.netease.com',
    )
    expect(wrapper.find('.api-key-input').exists()).toBe(true)
    expect(wrapper.find('.api-check-button').text()).toContain('检测')
    expect((wrapper.find('.api-model-select').element as HTMLSelectElement).value).toBe(
      'doubao-seed-1-6-vision-250815',
    )
  })

  it('keeps settings content scrollable inside the visible app area', () => {
    const wrapper = mount(SettingsModal)
    const dialog = wrapper.find('.dialog')

    expect(dialog.classes()).toContain('scrollable-dialog')
  })

  it('keeps default models when connection succeeds without a model list', async () => {
    vi.mocked(checkApiConnection).mockResolvedValue({
      ok: true,
      message: '连接成功',
      models: [],
    })
    const wrapper = mount(SettingsModal)
    await openSettingsTab(wrapper, 1)

    await wrapper.find('.api-check-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(checkApiConnection).toHaveBeenCalledWith({
      baseUrl: 'https://ai.leihuo.netease.com',
      apiKey: '',
    })
    const options = wrapper.findAll('.api-model-select option').map((option) => option.text())
    expect(options).toEqual([
      'doubao-seed-1-6-vision-250815',
      'gpt-5.4-mini',
      'qwen3.5-omni-plus',
      'qwen3-vl-plus',
    ])
    expect(wrapper.find('.api-status').text()).toContain('连接成功')
  })
})

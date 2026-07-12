import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsModal from '@/components/SettingsModal.vue'
import {
  checkAiProviderConnection,
  listAiProviders,
  saveAiProvider,
} from '@/lib/provider-ipc'
import { checkAppUpdate, installAppUpdate } from '@/lib/updater'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { open } from '@tauri-apps/plugin-dialog'
import {
  commitLegacyImport,
  discardLegacyImportPreview,
  inspectLegacyImport,
} from '@/lib/backup-ipc'

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
  readImportDir: vi.fn().mockResolvedValue([]),
  downloadImage: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@/lib/provider-ipc', () => ({
  listAiProviders: vi.fn(),
  saveAiProvider: vi.fn(),
  checkAiProviderConnection: vi.fn(),
}))

vi.mock('@/lib/backup-ipc', () => ({
  inspectLegacyImport: vi.fn(),
  commitLegacyImport: vi.fn(),
  discardLegacyImportPreview: vi.fn(),
}))

vi.mock('@/lib/updater', () => ({
  checkAppUpdate: vi.fn(),
  installAppUpdate: vi.fn(),
}))

async function openSettingsTab(wrapper: ReturnType<typeof mount>, index: number) {
  await wrapper.findAll('.settings-tab')[index].trigger('click')
}

const reverseImageProvider = {
  id: 'reverse-image' as const,
  kind: 'reverse-image' as const,
  displayName: '图片反推',
  baseUrl: 'https://ai.leihuo.netease.com',
  modelsUrl: 'https://ai.leihuo.netease.com/v1/models',
  chatCompletionsUrl: 'https://ai.leihuo.netease.com/v1/chat/completions',
  defaultModel: 'doubao-seed-1-6-vision-250815',
  availableModels: ['doubao-seed-1-6-vision-250815', 'gpt-5.4-mini'],
  probedModel: null,
  structuredMode: null,
  interactiveCompatible: null,
  boundHost: 'https://ai.leihuo.netease.com',
  needsCredentials: true,
  configRevision: 1,
  capabilityRevision: 1,
}

const storyboardProvider = {
  id: 'storyboard' as const,
  kind: 'storyboard' as const,
  displayName: '故事板 Agent',
  baseUrl: 'https://story.example.com',
  modelsUrl: 'https://story.example.com/v1/models',
  chatCompletionsUrl: 'https://story.example.com/v1/chat/completions',
  defaultModel: 'glm-5.2',
  availableModels: ['glm-5.2', 'glm-4.7'],
  probedModel: null,
  structuredMode: null,
  interactiveCompatible: null,
  boundHost: 'https://story.example.com',
  needsCredentials: true,
  configRevision: 1,
  capabilityRevision: 1,
  temperature: 0.7,
  contextWindowTokens: 16000,
}

const legacyImportPreview = {
  token: '2dd6bcf1-262f-4cf1-a507-2d375750759c',
  promptCount: 2,
  categoryCount: 1,
  hasApiKey: true,
  credentialConflict: false,
  warnings: [],
}

const legacyImportCommit = {
  library: {
    version: 2,
    categories: [],
    prompts: [],
    settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' as const },
  },
  promptsImported: 2,
  categoriesImported: 1,
  warnings: [],
}

describe('SettingsModal', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(isEnabled).mockResolvedValue(false)
    vi.mocked(listAiProviders).mockResolvedValue([reverseImageProvider])
    vi.mocked(open).mockResolvedValue(null)
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

  it('loads public reverse-provider settings with a write-only password input', async () => {
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    expect((wrapper.find('.api-base-url-input').element as HTMLInputElement).value).toBe(
      'https://ai.leihuo.netease.com',
    )
    const keyInput = wrapper.find('.api-key-input').element as HTMLInputElement
    expect(keyInput.value).toBe('')
    expect(keyInput.getAttribute('placeholder')).toContain('留空表示不修改')
    expect((wrapper.find('.api-models-url-input').element as HTMLInputElement).value).toContain(
      '/v1/models',
    )
    expect(wrapper.find('.api-check-button').text()).toContain('检测')
    expect((wrapper.find('.api-model-select').element as HTMLSelectElement).value).toBe(
      'doubao-seed-1-6-vision-250815',
    )
  })

  it('switches API settings to the independent storyboard provider', async () => {
    vi.mocked(listAiProviders).mockResolvedValue([reverseImageProvider, storyboardProvider])
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    const selector = wrapper.get('[data-field="api-provider"]')
    expect((selector.element as HTMLSelectElement).value).toBe('reverse-image')

    await selector.setValue('storyboard')

    expect((wrapper.find('.api-base-url-input').element as HTMLInputElement).value).toBe(
      'https://story.example.com',
    )
    expect((wrapper.get('[data-field="api-temperature"]').element as HTMLInputElement).value).toBe('0.7')
    expect((wrapper.get('[data-field="api-context-window"]').element as HTMLInputElement).value).toBe('16000')
    expect((wrapper.find('.api-model-select').element as HTMLSelectElement).value).toBe('glm-5.2')
  })

  it('keeps settings content scrollable inside the visible app area', () => {
    const wrapper = mount(SettingsModal)
    const dialog = wrapper.find('.dialog')

    expect(dialog.classes()).toContain('scrollable-dialog')
  })

  it('checks the stored provider by ID and keeps its models when probing returns none', async () => {
    vi.mocked(checkAiProviderConnection).mockResolvedValue({
      ok: true,
      message: '连接成功',
      models: [],
    })
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    await wrapper.find('.api-check-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(checkAiProviderConnection).toHaveBeenCalledWith('reverse-image')
    const options = wrapper.findAll('.api-model-select option').map((option) => option.text())
    expect(options).toEqual(['doubao-seed-1-6-vision-250815', 'gpt-5.4-mini'])
    expect(wrapper.find('.api-status').text()).toContain('连接成功')
  })

  it('saves the password once and clears the local input after saving', async () => {
    vi.mocked(saveAiProvider).mockResolvedValue({
      ...reverseImageProvider,
      needsCredentials: false,
    })
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    await wrapper.find('.api-key-input').setValue('TEST_ONLY_WRITE_ONCE')
    await wrapper.find('.api-save-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(saveAiProvider).toHaveBeenCalledWith({
      provider: {
        id: 'reverse-image',
        kind: 'reverse-image',
        displayName: '图片反推',
        baseUrl: 'https://ai.leihuo.netease.com',
        modelsUrl: 'https://ai.leihuo.netease.com/v1/models',
        chatCompletionsUrl: 'https://ai.leihuo.netease.com/v1/chat/completions',
        defaultModel: 'doubao-seed-1-6-vision-250815',
        confirmCrossOrigin: false,
      },
      apiKey: 'TEST_ONLY_WRITE_ONCE',
    })
    expect((wrapper.find('.api-key-input').element as HTMLInputElement).value).toBe('')
  })

  it('clears the password input when saving the provider fails', async () => {
    vi.mocked(saveAiProvider).mockRejectedValue(new Error('network failure'))
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    await wrapper.find('.api-key-input').setValue('TEST_ONLY_CLEAR_ON_FAILURE')
    await wrapper.find('.api-save-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect((wrapper.find('.api-key-input').element as HTMLInputElement).value).toBe('')
  })

  it('clears the password input when the provider has not loaded', async () => {
    vi.mocked(listAiProviders).mockResolvedValue([])
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 1)

    await wrapper.find('.api-key-input').setValue('TEST_ONLY_CLEAR_WITHOUT_PROVIDER')
    await wrapper.find('.api-save-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect((wrapper.find('.api-key-input').element as HTMLInputElement).value).toBe('')
  })

  it('stages a legacy library before committing the sanitized import', async () => {
    vi.mocked(open).mockResolvedValue('C:\\Users\\Felix\\Downloads\\legacy.zip')
    vi.mocked(inspectLegacyImport).mockResolvedValue(legacyImportPreview)
    vi.mocked(commitLegacyImport).mockResolvedValue(legacyImportCommit)
    const wrapper = mount(SettingsModal)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await openSettingsTab(wrapper, 3)

    await wrapper.find('[data-action="inspect-legacy-import"]').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(inspectLegacyImport).toHaveBeenCalledWith('C:\\Users\\Felix\\Downloads\\legacy.zip')
    expect(wrapper.find('.legacy-import-preview').exists()).toBe(true)

    await wrapper.find('[data-action="commit-legacy-import"]').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(commitLegacyImport).toHaveBeenCalledWith(legacyImportPreview.token, false)
    expect(discardLegacyImportPreview).not.toHaveBeenCalled()
  })
})

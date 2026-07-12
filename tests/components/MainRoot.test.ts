import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import MainRoot from '@/components/MainRoot.vue'
import { getStartupStatus } from '@/lib/startup-ipc'

vi.mock('@/components/ReadyApp.vue', () => ({
  default: {
    name: 'ReadyApp',
    template: '<div data-test="main-app">主应用</div>',
  },
}))

vi.mock('@/lib/startup-ipc', () => ({
  getStartupStatus: vi.fn(),
}))

describe('MainRoot', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('only mounts the main application after startup is ready', async () => {
    vi.mocked(getStartupStatus).mockResolvedValue({
      state: 'ready',
      migrationSummary: null,
    })

    const wrapper = mount(MainRoot)

    expect(wrapper.find('[data-test="main-app"]').exists()).toBe(false)
    await flushPromises()
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await flushPromises()

    expect(wrapper.find('[data-test="main-app"]').exists()).toBe(true)
  })

  it('defers business application and Pinia setup until startup is ready', () => {
    const mainRootSource = readFileSync(resolve(process.cwd(), 'src/components/MainRoot.vue'), 'utf8')
    const mainSource = readFileSync(resolve(process.cwd(), 'src/main.ts'), 'utf8')

    expect(mainRootSource).not.toMatch(/import\s+App\s+from/)
    expect(mainRootSource).toContain("import('@/components/ReadyApp.vue')")
    expect(mainSource).not.toMatch(/from ['"]pinia['"]/)
    expect(mainSource).not.toContain('.use(createPinia())')
  })

  it('shows recovery instead of mounting the main application', async () => {
    vi.mocked(getStartupStatus).mockResolvedValue({
      state: 'recovery',
      message: '本地数据需要恢复。',
      backupPaths: ['C:/data/library-backup.json'],
    })

    const wrapper = mount(MainRoot)
    await flushPromises()

    expect(wrapper.find('[data-test="main-app"]').exists()).toBe(false)
    expect(wrapper.find('.recovery-page').text()).toContain('本地数据需要恢复。')
  })

  it('shows a migration summary and closes it after acknowledgement', async () => {
    vi.mocked(getStartupStatus).mockResolvedValue({
      state: 'ready',
      migrationSummary: {
        promptsMigrated: 8,
        favoritesDefaulted: 3,
        ordersRebuilt: 2,
        backupPath: 'C:/data/library-v0.json',
        warnings: ['已保留一份备份。'],
      },
    })

    const wrapper = mount(MainRoot)
    await flushPromises()
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await flushPromises()

    expect(wrapper.find('.migration-summary-dialog').exists()).toBe(true)

    await wrapper.findComponent({ name: 'MigrationSummaryDialog' }).vm.$emit('acknowledged')

    expect(wrapper.find('.migration-summary-dialog').exists()).toBe(false)
  })
})

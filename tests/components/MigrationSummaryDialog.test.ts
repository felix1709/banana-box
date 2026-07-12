import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import MigrationSummaryDialog from '@/components/MigrationSummaryDialog.vue'
import { acknowledgeMigrationSummary } from '@/lib/startup-ipc'

vi.mock('@/lib/startup-ipc', () => ({
  acknowledgeMigrationSummary: vi.fn(),
}))

const summary = {
  promptsMigrated: 8,
  favoritesDefaulted: 3,
  ordersRebuilt: 2,
  backupPath: 'C:/data/library-v0.json',
  warnings: ['已保留一份备份。'],
}

describe('MigrationSummaryDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows migration counts, backup path, and warnings', () => {
    const wrapper = mount(MigrationSummaryDialog, {
      props: { summary },
    })

    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('8')
    expect(wrapper.text()).toContain('C:/data/library-v0.json')
    expect(wrapper.text()).toContain('已保留一份备份。')
  })

  it('closes only after the acknowledgement succeeds', async () => {
    vi.mocked(acknowledgeMigrationSummary).mockResolvedValue(undefined)
    const wrapper = mount(MigrationSummaryDialog, {
      props: { summary },
    })

    await wrapper.find('.migration-summary-confirm').trigger('click')

    expect(acknowledgeMigrationSummary).toHaveBeenCalledOnce()
    expect(wrapper.emitted('acknowledged')).toHaveLength(1)
  })
})

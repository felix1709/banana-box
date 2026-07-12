import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import RecoveryPage from '@/components/RecoveryPage.vue'

describe('RecoveryPage', () => {
  it('keeps recovery information visible without destructive actions', () => {
    const wrapper = mount(RecoveryPage, {
      props: {
        status: {
          state: 'recovery',
          message: '数据校验未通过，请先保留备份。',
          backupPaths: ['C:/data/library-v0.json', 'C:/data/library-v0.json.bak'],
        },
      },
    })

    expect(wrapper.text()).toContain('数据校验未通过，请先保留备份。')
    expect(wrapper.findAll('.recovery-path')).toHaveLength(2)
    expect((wrapper.find('.recovery-path').element as HTMLTextAreaElement).readOnly).toBe(true)
    expect(wrapper.find('.recovery-retry').text()).toContain('重新检查')
    expect(wrapper.find('[data-test="recovery-delete"]').exists()).toBe(false)
  })
})

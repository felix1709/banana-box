import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import SyncStatusIndicator from '@/components/cloud/SyncStatusIndicator.vue'
import { useSyncStatusStore } from '@/stores/syncStatus'

describe('SyncStatusIndicator', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows conflict count before claiming sync is clean', () => {
    const sync = useSyncStatusStore()
    sync.state = 'conflict'
    sync.conflicts = ['record-1']

    const wrapper = mount(SyncStatusIndicator)

    expect(wrapper.text()).toContain('冲突 1')
  })
})

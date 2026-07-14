import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CloudMigrationDialog from '@/components/cloud/CloudMigrationDialog.vue'
import { useAuthStore } from '@/stores/auth'
import { useCloudMigrationStore } from '@/stores/cloudMigration'
import { useLibraryStore } from '@/stores/library'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('CloudMigrationDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows a compact local cloud and pending sync summary and starts migration', async () => {
    useAuthStore().user = { id: 'user-1' } as never
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: null,
        tags: [],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 1,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    const migration = useCloudMigrationStore()
    migration.cloudSummary = {
      categories: 0,
      prompts: 0,
      projects: 0,
      dailyDays: 0,
      dailyTasks: 0,
      loaded: true,
      loading: false,
    }
    migration.migrateNow = vi.fn()

    const wrapper = mount(CloudMigrationDialog)

    expect(wrapper.text()).toContain('发现本地数据')
    expect(wrapper.find('.cloud-migration-table').exists()).toBe(false)
    expect(wrapper.get('[data-migration-stat="local"]').text()).toContain('本地')
    expect(wrapper.get('[data-migration-stat="local"]').text()).toContain('1')
    expect(wrapper.get('[data-migration-stat="cloud"]').text()).toContain('云端')
    expect(wrapper.get('[data-migration-stat="cloud"]').text()).toContain('0')
    expect(wrapper.get('[data-migration-stat="pending"]').text()).toContain('待同步')
    expect(wrapper.get('[data-migration-stat="pending"]').text()).toContain('1')
    expect(wrapper.text()).toContain('请选择是否同步到云端')
    expect(wrapper.text()).toContain('保持本地')
    expect(wrapper.text()).toContain('稍后处理')
    expect(wrapper.text()).toContain('同步到云端')

    await wrapper.get('[data-action="cloud-migrate-now"]').trigger('click')
    expect(migration.migrateNow).toHaveBeenCalled()
  })

  it('keeps the dialog visible while syncing or showing an upload error', async () => {
    useAuthStore().user = { id: 'user-1' } as never
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: null,
        tags: [],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 1,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    const migration = useCloudMigrationStore()
    migration.status = 'error'
    migration.error = '上传失败'

    const wrapper = mount(CloudMigrationDialog)

    expect(wrapper.find('.cloud-migration-dialog').exists()).toBe(true)
    expect(wrapper.text()).toContain('上传失败')

    migration.status = 'running'
    migration.error = ''
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('正在同步')
  })

  it('waits for the active workspace before loading cloud counts', async () => {
    const auth = useAuthStore()
    auth.user = { id: 'user-1' } as never
    auth.client = {} as never
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [{
        id: 'prompt-1',
        title: '标题',
        content: '内容',
        categoryId: null,
        tags: [],
        image: null,
        favorite: false,
        order: 0,
        createdAt: 1,
        updatedAt: 1,
      }],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    const migration = useCloudMigrationStore()
    const loadCloudSummary = vi.spyOn(migration, 'loadCloudSummary').mockResolvedValue()

    const wrapper = mount(CloudMigrationDialog)
    await wrapper.vm.$nextTick()

    expect(loadCloudSummary).not.toHaveBeenCalled()

    useWorkspacesStore().activeWorkspaceId = 'workspace-1'
    await wrapper.vm.$nextTick()

    expect(loadCloudSummary).toHaveBeenCalledTimes(1)
  })
})

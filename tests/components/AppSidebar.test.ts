import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import AppSidebar from '@/components/AppSidebar.vue'
import { useUiStore } from '@/stores/ui'
import { FAVORITES_CATEGORY_ID, useLibraryStore } from '@/stores/library'

describe('AppSidebar', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows the three top-level tools and switches between them', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    expect(wrapper.text()).toContain('提示词库')
    expect(wrapper.text()).toContain('反推图片')
    expect(wrapper.text()).toContain('快速压缩')

    await wrapper.find('[data-tool="reverse-image"]').trigger('click')
    expect(ui.activeTool).toBe('reverse-image')

    await wrapper.find('[data-tool="compression"]').trigger('click')
    expect(ui.activeTool).toBe('compression')
  })

  it('shows prompt categories under the prompt library tool only when prompts are active', async () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [{ id: 'style', name: 'Style', color: '#22c55e', order: 0 }],
      prompts: [],
      settings: {
        hotkey: 'Ctrl+Shift+B',
        theme: 'auto',
        apiBaseUrl: 'https://ai.leihuo.netease.com',
        apiKey: '',
        reverseModel: 'doubao-seed-1-6-vision-250815',
        availableReverseModels: ['doubao-seed-1-6-vision-250815'],
      },
    })
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    expect(wrapper.find('.sidebar-category-list').exists()).toBe(true)
    expect(wrapper.text()).toContain('Style')

    await wrapper.find('[data-tool="compression"]').trigger('click')
    expect(ui.activeTool).toBe('compression')
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(false)
  })

  it('puts a red plus new prompt action next to the prompt library tool', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    const createButton = wrapper.find('[data-action="create-prompt"]')

    expect(createButton.exists()).toBe(true)
    expect(createButton.classes()).toContain('create-prompt-button')
    expect(createButton.text()).toBe('+')

    await createButton.trigger('click')

    expect(ui.editorOpen).toBe(true)
    expect(ui.editingPromptId).toBeNull()
  })

  it('does not put plus actions next to the reverse image or compression tools', () => {
    const wrapper = mount(AppSidebar)

    expect(wrapper.findAll('[data-action="create-prompt"]')).toHaveLength(1)
    expect(wrapper.find('[data-tool-row="reverse-image"] [data-action="create-prompt"]').exists()).toBe(
      false,
    )
    expect(wrapper.find('[data-tool-row="compression"] [data-action="create-prompt"]').exists()).toBe(
      false,
    )
  })

  it('renders prompt categories in compact sidebar styling', () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [{ id: 'style', name: 'Style', color: '#22c55e', order: 0 }],
      prompts: [],
      settings: {
        hotkey: 'Ctrl+Shift+B',
        theme: 'auto',
        apiBaseUrl: 'https://ai.leihuo.netease.com',
        apiKey: '',
        reverseModel: 'doubao-seed-1-6-vision-250815',
        availableReverseModels: ['doubao-seed-1-6-vision-250815'],
      },
    })

    const wrapper = mount(AppSidebar)

    expect(wrapper.find('.sidebar-category-list .tree.compact').exists()).toBe(true)
  })

  it('shows a fixed favorites category that cannot be deleted', async () => {
    const wrapper = mount(AppSidebar)
    const lib = useLibraryStore()

    const favorite = wrapper.find('.favorite-category')
    expect(favorite.exists()).toBe(true)
    expect(favorite.find('.del').exists()).toBe(false)

    await favorite.trigger('click')

    expect(lib.currentCategoryId).toBe(FAVORITES_CATEGORY_ID)
  })
})

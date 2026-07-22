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

  it('shows the top-level tools and switches between them', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    expect(wrapper.text()).toContain('提示词库')
    expect(wrapper.text()).toContain('反推图片')
    expect(wrapper.text()).toContain('快速压缩')

    expect(wrapper.findAll('.tool-button')[0].attributes('data-tool')).toBe('shared-library')

    await wrapper.find('[data-tool="shared-library"]').trigger('click')
    expect(ui.activeTool).toBe('shared-library')

    await wrapper.find('[data-tool="reverse-image"]').trigger('click')
    expect(ui.activeTool).toBe('reverse-image')

    await wrapper.find('[data-tool="compression"]').trigger('click')
    expect(ui.activeTool).toBe('compression')

    await wrapper.find('[data-tool="projects"]').trigger('click')
    expect(ui.activeTool).toBe('projects')
  })

  it('toggles all prompt categories from the prompt library button', async () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [{ id: 'style', name: 'Style', color: '#22c55e', order: 0 }],
      prompts: [],
      settings: {
        hotkey: 'Ctrl+Shift+B',
        theme: 'auto',
      },
    })
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    expect(wrapper.find('.sidebar-category-list').exists()).toBe(false)

    await wrapper.find('[data-tool="prompts"]').trigger('click')
    expect(ui.activeTool).toBe('prompts')
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(true)
    expect(wrapper.text()).toContain('Style')
    expect(wrapper.find('[data-tool="prompts"]').attributes('aria-expanded')).toBe('true')

    await wrapper.find('[data-tool="prompts"]').trigger('click')
    expect(ui.activeTool).toBe('prompts')
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(false)
    expect(wrapper.find('[data-tool="prompts"]').attributes('aria-expanded')).toBe('false')
  })

  it('collapses prompt categories when switching to another tool', async () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [{ id: 'style', name: 'Style', color: '#22c55e', order: 0 }],
      prompts: [],
      settings: {
        hotkey: 'Ctrl+Shift+B',
        theme: 'auto',
      },
    })
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    await wrapper.find('[data-tool="prompts"]').trigger('click')
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(true)
    await wrapper.find('[data-tool="compression"]').trigger('click')

    expect(ui.activeTool).toBe('compression')
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(false)
  })

  it('switches to the daily task tool', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    await wrapper.get('[data-tool="daily-tasks"]').trigger('click')

    expect(ui.activeTool).toBe('daily-tasks')
  })

  it('hides the unfinished storyboard tool from the sidebar', () => {
    const wrapper = mount(AppSidebar)

    expect(wrapper.find('[data-tool="storyboard"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('故事板')
  })

  it('switches to the PI-Web tool', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    await wrapper.get('[data-tool="pi-web"]').trigger('click')

    expect(ui.activeTool).toBe('pi-web')
    expect(wrapper.text()).toContain('PI-Web')
  })

  it('puts a compact icon-only prompt action next to the prompt library tool', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    const createButton = wrapper.find('[data-action="create-prompt"]')

    expect(createButton.exists()).toBe(true)
    expect(createButton.classes()).toContain('create-prompt-button')
    expect(createButton.attributes('title')).toBe('新增提示词')
    expect(createButton.attributes('aria-label')).toBe('新增提示词')
    expect(createButton.find('svg.lucide-plus').exists()).toBe(true)
    expect(createButton.text()).toBe('')

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

  it('renders prompt categories in compact sidebar styling', async () => {
    const lib = useLibraryStore()
    lib.hydrate({
      version: 1,
      categories: [{ id: 'style', name: 'Style', color: '#22c55e', order: 0 }],
      prompts: [],
      settings: {
        hotkey: 'Ctrl+Shift+B',
        theme: 'auto',
      },
    })

    const wrapper = mount(AppSidebar)
    await wrapper.find('[data-tool="prompts"]').trigger('click')

    expect(wrapper.find('.sidebar-category-list .tree.compact').exists()).toBe(true)
  })

  it('shows a fixed favorites category that cannot be deleted', async () => {
    const wrapper = mount(AppSidebar)
    const lib = useLibraryStore()

    await wrapper.find('[data-tool="prompts"]').trigger('click')
    const favorite = wrapper.find('.favorite-category')
    expect(favorite.exists()).toBe(true)
    expect(favorite.find('.del').exists()).toBe(false)

    await favorite.trigger('click')

    expect(lib.currentCategoryId).toBe(FAVORITES_CATEGORY_ID)
  })
})

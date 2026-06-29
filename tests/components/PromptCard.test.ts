import { mount } from '@vue/test-utils'
import * as ipc from '@/lib/ipc'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PromptCard from '@/components/PromptCard.vue'
import { useLibraryStore } from '@/stores/library'
import type { Library, Prompt } from '@/types'

vi.mock('@/lib/ipc', () => ({
  loadLibrary: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  saveImage: vi.fn().mockResolvedValue('images/new.png'),
  deleteImage: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
  exportLibrary: vi.fn(),
  importLibrary: vi.fn(),
  readImportDir: vi.fn(),
  downloadImage: vi.fn(),
}))

const prompt: Prompt = {
  id: 'p1',
  title: '总结助手',
  content: '请总结三点',
  categoryId: 'c1',
  tags: ['中文'],
  image: null,
  createdAt: 1,
  updatedAt: 1,
}

function seed(): Library {
  return {
    version: 1,
    categories: [
      { id: 'c1', name: '写作', color: '#f59e0b', order: 0 },
      { id: 'c2', name: '翻译', color: '#3b82f6', order: 1 },
    ],
    prompts: [{ ...prompt }],
    settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
  }
}

describe('PromptCard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    useLibraryStore().hydrate(seed())
    vi.clearAllMocks()
  })

  it('shows four actions after single click', async () => {
    const wrapper = mount(PromptCard, {
      attachTo: document.body,
      props: { prompt },
    })

    await wrapper.trigger('click')
    const actions = wrapper.findAll('.actions button').map((button) => button.text())
    expect(actions).toEqual(['删除', '编辑', '分类', '复制'])
  })

  it('does not expand before the left click is completed', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('pointerdown')

    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.find('.actions').exists()).toBe(false)
  })

  it('toggles between collapsed and expanded on repeated left clicks', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.find('.actions').exists()).toBe(false)

    await wrapper.trigger('click')

    expect(wrapper.classes()).toContain('expanded')
    expect(wrapper.find('.actions').exists()).toBe(true)

    await wrapper.trigger('click')

    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.find('.actions').exists()).toBe(false)
  })

  it('can be controlled by the parent expanded prop', async () => {
    const wrapper = mount(PromptCard, { props: { prompt, expanded: false } })

    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.find('.actions').exists()).toBe(false)

    await wrapper.setProps({ expanded: true })

    expect(wrapper.classes()).toContain('expanded')
    expect(wrapper.find('.actions').exists()).toBe(true)

    await wrapper.setProps({ expanded: false })

    expect(wrapper.classes()).toContain('collapsed')
    expect(wrapper.find('.actions').exists()).toBe(false)
  })

  it('keeps double click copy behavior', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('dblclick')

    expect(ipc.copyToClipboard).toHaveBeenCalledWith(prompt.content)
  })

  it('changes category from the action area', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const lib = useLibraryStore()

    await wrapper.trigger('click')
    await wrapper.findAll('.actions button')[2].trigger('click')
    await wrapper.findAll('.category-menu-item')[2].trigger('click')

    expect(lib.library.prompts[0].categoryId).toBe('c2')
  })

  it('opens a direct category menu from the category action', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')
    await wrapper.findAll('.actions button')[2].trigger('click')

    const menuItems = wrapper.findAll('.category-menu-item').map((button) => button.text())
    expect(wrapper.find('select.category-select').exists()).toBe(false)
    expect(wrapper.find('.category-menu').exists()).toBe(true)
    expect(menuItems).toEqual(['未分类', '写作', '翻译'])
    wrapper.unmount()
  })

  it('uses a square thumbnail area that can pick an image', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const file = new File(['fake'], 'ref.png', { type: 'image/png' })

    expect(wrapper.find('.thumb-zone').exists()).toBe(true)
    expect(wrapper.find('.thumb-zone').classes()).toContain('empty')

    Object.defineProperty(wrapper.find('input[type="file"]').element, 'files', {
      value: [file],
      configurable: true,
    })
    await wrapper.find('input[type="file"]').trigger('change')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(ipc.saveImage).toHaveBeenCalledWith(expect.any(Array), 'png')
    expect(useLibraryStore().library.prompts[0].image).toBe('images/new.png')
  })

  it('accepts a dropped image on the card', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const file = new File(['fake'], 'dropped.webp', { type: 'image/webp' })

    await wrapper.trigger('drop', {
      dataTransfer: {
        files: [file],
      },
    })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(ipc.saveImage).toHaveBeenCalledWith(expect.any(Array), 'webp')
    expect(useLibraryStore().library.prompts[0].image).toBe('images/new.png')
  })

  it('accepts a pasted image after actions are visible', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const file = new File(['fake'], 'pasted.jpg', { type: 'image/jpeg' })
    const paste = new Event('paste') as ClipboardEvent

    Object.defineProperty(paste, 'clipboardData', {
      value: { files: [file] },
    })

    await wrapper.trigger('click')
    document.dispatchEvent(paste)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(ipc.saveImage).toHaveBeenCalledWith(expect.any(Array), 'jpg')
    expect(useLibraryStore().library.prompts[0].image).toBe('images/new.png')
  })

  it('previews the original image when an existing thumbnail is clicked', async () => {
    vi.mocked(ipc.readImageBytes).mockResolvedValue('blob:preview')
    const withImage = { ...prompt, image: 'images/existing.png' }
    const wrapper = mount(PromptCard, { props: { prompt: withImage } })

    await wrapper.find('.thumb-zone').trigger('click')

    expect(wrapper.vm).toBeTruthy()
    expect(useLibraryStore().library.prompts[0].image).toBeNull()
    const { useUiStore } = await import('@/stores/ui')
    expect(useUiStore().previewImage).toBe('images/existing.png')
  })
})

import { mount } from '@vue/test-utils'
import * as ipc from '@/lib/ipc'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
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
  favorite: false,
  order: 0,
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
    vi.useRealTimers()
    setActivePinia(createPinia())
    useLibraryStore().hydrate(seed())
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
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

  it('prevents text selection when the left button starts a long press', () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const event = new PointerEvent('pointerdown', {
      bubbles: true,
      cancelable: true,
      button: 0,
    })

    wrapper.element.dispatchEvent(event)

    expect(event.defaultPrevented).toBe(true)
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

  it('copies on double click and leaves the prompt collapsed', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    wrapper.element.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 1 }))
    await wrapper.vm.$nextTick()
    wrapper.element.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 2 }))
    await wrapper.vm.$nextTick()
    await wrapper.trigger('dblclick')

    expect(ipc.copyToClipboard).toHaveBeenCalledWith(prompt.content)
    expect(wrapper.classes()).toContain('collapsed')
  })

  it('shows the full prompt content after single click expands the card', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')

    expect(wrapper.classes()).toContain('expanded')
    expect(wrapper.classes()).toContain('expanded-auto-height-card')
    expect(wrapper.classes()).toContain('flow-expanded-card')
    expect(wrapper.classes()).not.toContain('fixed-size-prompt-card')
    expect(wrapper.find('.content').classes()).toContain('full-content')
    expect(wrapper.find('.tags').classes()).toContain('full-tags')
  })

  it('uses normal document flow for expanded content so following cards are pushed down', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')

    expect(wrapper.find('.card-main').classes()).toContain('flow-card-main')
    expect(wrapper.find('.text-pane').classes()).toContain('flow-text-pane')
    expect(wrapper.find('.content').classes()).toContain('flow-content')
  })

  it('lets individual tags expand with the prompt card', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')

    expect(wrapper.find('.tag').classes()).toContain('full-tag')
  })

  it('toggles favorite from the star button without expanding the card', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const lib = useLibraryStore()

    await wrapper.find('.favorite-button').trigger('mousedown')
    await wrapper.find('.favorite-button').trigger('click')

    expect(lib.library.prompts[0].favorite).toBe(true)
    expect(wrapper.find('.favorite-button').classes()).toContain('active')
    expect(wrapper.classes()).toContain('collapsed')
  })

  it('renders favorite as an icon-only star without a circular outline', () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    expect(wrapper.find('.favorite-button').classes()).toContain('icon-only-star')
  })

  it('enables drag sorting only after a long press', async () => {
    vi.useFakeTimers()
    const wrapper = mount(PromptCard, { props: { prompt } })

    expect(wrapper.attributes('draggable')).toBe('false')
    expect(wrapper.classes()).not.toContain('drag-ready')

    await wrapper.trigger('pointerdown')

    await vi.advanceTimersByTimeAsync(410)

    expect(wrapper.attributes('draggable')).toBe('false')
    expect(wrapper.classes()).toContain('drag-ready')
    expect(wrapper.classes()).toContain('drag-floating')
    expect(wrapper.emitted('sort-start')?.[0]).toEqual(['p1'])
    vi.useRealTimers()
  })

  it('moves the card visually with the mouse during long press sorting', async () => {
    vi.useFakeTimers()
    const wrapper = mount(PromptCard, { props: { prompt } })
    vi.spyOn(wrapper.element, 'getBoundingClientRect').mockReturnValue({
      left: 100,
      top: 200,
      width: 320,
      height: 104,
      right: 420,
      bottom: 304,
      x: 100,
      y: 200,
      toJSON: () => ({}),
    })

    wrapper.element.dispatchEvent(
      new PointerEvent('pointerdown', { bubbles: true, button: 0, clientX: 120, clientY: 220 }),
    )
    await wrapper.vm.$nextTick()
    await vi.advanceTimersByTimeAsync(410)
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 150, clientY: 260 }))
    await wrapper.vm.$nextTick()

    expect(wrapper.attributes('style')).toContain('position: fixed')
    expect(wrapper.attributes('style')).toContain('left: 100px')
    expect(wrapper.attributes('style')).toContain('top: 200px')
    expect(wrapper.attributes('style')).toContain('width: 320px')
    expect(wrapper.attributes('style')).toContain('height: 104px')
    expect(wrapper.attributes('style')).toContain('translate3d(30px, 40px, 0px)')
    vi.useRealTimers()
  })

  it('emits sort-over from document pointer movement during a long press sort', async () => {
    vi.useFakeTimers()
    const wrapper = mount(PromptCard, {
      attachTo: document.body,
      props: { prompt },
    })
    const target = document.createElement('div')
    target.dataset.promptCardId = 'p2'
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: vi.fn(() => target),
    })

    await wrapper.trigger('pointerdown')
    await vi.advanceTimersByTimeAsync(410)
    document.dispatchEvent(new PointerEvent('pointermove', { clientX: 20, clientY: 30 }))

    expect(wrapper.emitted('sort-over')?.[0]).toEqual(['p2'])

    wrapper.unmount()
    vi.useRealTimers()
  })

  it('emits sort-end when a long press sort is released anywhere', async () => {
    vi.useFakeTimers()
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('pointerdown')
    await vi.advanceTimersByTimeAsync(410)
    document.dispatchEvent(new PointerEvent('pointerup'))
    await wrapper.vm.$nextTick()

    expect(wrapper.emitted('sort-end')?.[0]).toEqual([])
    expect(wrapper.classes()).not.toContain('drag-ready')
    vi.useRealTimers()
  })

  it('emits sort-over when a long-pressed prompt moves over this card', async () => {
    const wrapper = mount(PromptCard, {
      props: { prompt, sortingPromptId: 'p2' },
    })

    await wrapper.trigger('pointerenter')

    expect(wrapper.emitted('sort-over')?.[0]).toEqual(['p1'])
  })

  it('keeps native dragging disabled because sorting uses pointer movement', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const dataTransfer = {
      setData: vi.fn(),
    }

    await wrapper.trigger('dragstart', { dataTransfer })

    expect(dataTransfer.setData).not.toHaveBeenCalled()
  })

  it('emits reorder-before when another prompt is dropped on the card', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })
    const dataTransfer = {
      getData: vi.fn(() => 'p2'),
    }

    await wrapper.trigger('drop', { dataTransfer })

    expect(wrapper.emitted('reorder-before')?.[0]).toEqual(['p2'])
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

  it('keeps the expanded card overflow visible so the category menu can be clicked', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')

    expect(wrapper.classes()).toContain('category-menu-can-overflow')
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

  it('shows existing thumbnails without crop styling', async () => {
    vi.mocked(ipc.readImageBytes).mockResolvedValue('blob:preview')
    const withImage = { ...prompt, image: 'images/existing.png' }

    const wrapper = mount(PromptCard, { props: { prompt: withImage } })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.thumb-zone').exists()).toBe(true)
    expect(wrapper.find('.thumb-zone').classes()).toContain('fit-box')
    expect(wrapper.find('.thumb-zone').classes()).toContain('prompt-preview-thumb')
    expect(wrapper.find('img.thumb').classes()).toContain('fit-contain')
  })

  it('keeps each prompt card content at a fixed three-line size', () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    expect(wrapper.classes()).toContain('fixed-size-prompt-card')
    expect(wrapper.find('.card-main').classes()).toContain('fixed-card-layout')
    expect(wrapper.find('.title').exists()).toBe(true)
    expect(wrapper.find('.content').classes()).toContain('fixed-three-line-content')
  })

  it('uses auto-height card layout after the card is expanded', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    await wrapper.trigger('click')

    expect(wrapper.classes()).not.toContain('fixed-size-prompt-card')
    expect(wrapper.find('.card-main').classes()).toContain('fixed-card-layout')
    expect(wrapper.find('.thumb-zone').classes()).toContain('prompt-preview-thumb')
  })

  it('keeps the preview thumbnail anchored when the card is clicked open', async () => {
    const wrapper = mount(PromptCard, { props: { prompt } })

    expect(wrapper.find('.thumb-zone').classes()).toContain('stable-preview-thumb')

    await wrapper.trigger('click')

    expect(wrapper.find('.thumb-zone').classes()).toContain('stable-preview-thumb')
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

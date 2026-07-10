import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from '@/App.vue'
import PromptCard from '@/components/PromptCard.vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'

let floatingDropHandler: ((event: { payload: unknown }) => void) | null = null
const coreApi = vi.hoisted(() => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}))
const windowApi = vi.hoisted(() => ({
  startResizeDragging: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((eventName: string, handler: (event: { payload: unknown }) => void) => {
    if (eventName === 'floating-file-dropped') floatingDropHandler = handler
    return Promise.resolve(vi.fn())
  }),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => windowApi),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: coreApi.invoke,
}))

vi.mock('@/lib/ipc', () => ({
  loadLibrary: vi.fn().mockResolvedValue({
    version: 1,
    categories: [],
    prompts: [],
    settings: {
      hotkey: 'Ctrl+Shift+B',
      theme: 'auto',
      apiBaseUrl: 'https://ai.leihuo.netease.com',
      apiKey: '',
      reverseModel: 'doubao-seed-1-6-vision-250815',
      availableReverseModels: [
        'doubao-seed-1-6-vision-250815',
        'gpt-5.4-mini',
        'qwen3.5-omni-plus',
        'qwen3-vl-plus',
      ],
    },
  }),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
}))

describe('App', () => {
  beforeEach(() => {
    vi.useRealTimers()
    setActivePinia(createPinia())
    floatingDropHandler = null
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('starts dragging the frameless main window from the dedicated drag strip', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    await wrapper.find('.window-drag-strip').trigger('mousedown')

    expect(coreApi.invoke).toHaveBeenCalledWith('begin_main_window_drag')
  })

  it('marks and highlights the drag strip while pressing it', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.window-drag-marker').exists()).toBe(true)

    await wrapper.find('.window-drag-strip').trigger('mousedown')
    expect(wrapper.find('.window-drag-strip').classes()).toContain('window-drag-strip-active')

    await wrapper.find('.window-drag-strip').trigger('mouseup')
    expect(wrapper.find('.window-drag-strip').classes()).not.toContain('window-drag-strip-active')
  })

  it('shows resize handles around the frameless window', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.findAll('.window-resize-handle')).toHaveLength(8)
    expect(wrapper.find('.window-resize-handle-east').exists()).toBe(true)
    expect(wrapper.find('.window-resize-handle-south-east').exists()).toBe(true)
  })

  it('starts protected window resizing and highlights the active edge', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    coreApi.invoke.mockClear()

    const eastHandle = wrapper.find('.window-resize-handle-east')
    await eastHandle.trigger('mousedown')

    expect(coreApi.invoke).toHaveBeenCalledWith('begin_main_window_resize')
    expect(windowApi.startResizeDragging).toHaveBeenCalledWith('East')
    expect(eastHandle.classes()).toContain('window-resize-handle-active')

    await eastHandle.trigger('mouseup')
    expect(eastHandle.classes()).not.toContain('window-resize-handle-active')
  })

  it('lets the app shell resize with the Tauri webview instead of staying at the launch size', () => {
    const source = readFileSync(resolve(process.cwd(), 'src/App.vue'), 'utf8')
    const styleBlock = source.match(/\.app\s*\{(?<content>[\s\S]*?)\n\}/)?.groups?.content ?? ''

    expect(styleBlock).toContain('width: 100vw')
    expect(styleBlock).toContain('height: 100vh')
    expect(styleBlock).not.toContain('width: 720px')
    expect(styleBlock).not.toContain('height: 520px')
  })

  it('does not start window dragging from topbar clicks or interactive controls', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    await wrapper.find('.topbar').trigger('mousedown')
    await wrapper.find('.search').trigger('mousedown')
    await wrapper.find('.btn').trigger('mousedown')

    expect(coreApi.invoke).not.toHaveBeenCalledWith('begin_main_window_drag')
  })

  it('pins the main window without starting a drag from the pin button', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    coreApi.invoke.mockClear()

    const pinButton = wrapper.find('.window-pin-button')
    expect(pinButton.exists()).toBe(true)

    await pinButton.trigger('mousedown')
    expect(coreApi.invoke).not.toHaveBeenCalledWith('begin_main_window_drag')

    await pinButton.trigger('click')

    expect(coreApi.invoke).toHaveBeenCalledWith('set_main_window_pinned', { pinned: true })
    expect(pinButton.classes()).toContain('window-pin-button-active')
  })

  it('renders the main window pin action as an icon-only button', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const pinButton = wrapper.find('.window-pin-button')

    expect(pinButton.text().trim()).toBe('')
    expect(pinButton.find('.window-pin-icon').exists()).toBe(true)
    expect(pinButton.find('.window-pin-icon').attributes('aria-hidden')).toBe('true')
  })

  it('keeps the new prompt action out of the topbar', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.topbar .btn.primary').exists()).toBe(false)
  })

  it('opens the floating action dialog when the float button drops a file', async () => {
    mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    floatingDropHandler?.({
      payload: {
        filePath: 'C:/tmp/photo.png',
        fileName: 'photo.png',
        fileType: 'image',
      },
    })

    const ui = useUiStore()
    expect(ui.panelVisible).toBe(true)
    expect(ui.floatingActionDialogOpen).toBe(true)
    expect(ui.floatingActionFile?.fileName).toBe('photo.png')
  })

  it('does not render the old separate category pane in the prompt library', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.category-pane').exists()).toBe(false)
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(true)
  })

  it('keeps the prompt library list in a dedicated scroll area', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.prompt-list').classes()).toContain('scrollable-panel')
  })

  it('uses an animated list container and a drop placeholder while sorting prompts', async () => {
    vi.useFakeTimers()
    const wrapper = mount(App)
    await vi.advanceTimersByTimeAsync(0)
    useLibraryStore().hydrate({
      version: 1,
      categories: [],
      prompts: [
        {
          id: 'p1',
          title: 'Prompt 1',
          content: 'Content 1',
          categoryId: null,
          tags: [],
          image: null,
          favorite: false,
          order: 0,
          createdAt: 1,
          updatedAt: 1,
        },
        {
          id: 'p2',
          title: 'Prompt 2',
          content: 'Content 2',
          categoryId: null,
          tags: [],
          image: null,
          favorite: false,
          order: 1,
          createdAt: 1,
          updatedAt: 1,
        },
      ],
      settings: { hotkey: 'Ctrl+Shift+B', theme: 'auto' },
    })
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.prompt-list').classes()).toContain('animated-prompt-list')
    expect(wrapper.find('.prompt-drop-placeholder').exists()).toBe(false)

    wrapper.findComponent(PromptCard).element.dispatchEvent(
      new PointerEvent('pointerdown', {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: 10,
        clientY: 10,
      }),
    )
    await vi.advanceTimersByTimeAsync(410)
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.prompt-drop-placeholder').exists()).toBe(true)

    document.dispatchEvent(new PointerEvent('pointerup'))
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.prompt-drop-placeholder').exists()).toBe(false)
  })
})

import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FloatButton from '@/components/FloatButton.vue'

const mocks = vi.hoisted(() => ({
  emitTo: vi.fn().mockResolvedValue(undefined),
  invoke: vi.fn().mockResolvedValue(undefined),
  onDragDropEvent: vi.fn(),
  startDragging: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: mocks.startDragging,
    onDragDropEvent: mocks.onDragDropEvent,
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  emitTo: mocks.emitTo,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mocks.invoke,
}))

describe('FloatButton', () => {
  beforeEach(() => {
    mocks.emitTo.mockClear()
    mocks.invoke.mockClear()
    mocks.onDragDropEvent.mockReset()
    mocks.onDragDropEvent.mockResolvedValue(vi.fn())
    mocks.startDragging.mockClear()
  })

  it('shows a banana button', () => {
    const wrapper = mount(FloatButton)

    expect(wrapper.text()).toBe('🍌')
  })

  it('accepts click without changing the visual label', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('click')

    expect(wrapper.text()).toBe('🍌')
  })

  it('emits an image drop action payload to the main window', async () => {
    const wrapper = mount(FloatButton)
    const file = new File(['fake'], 'photo.png', { type: 'image/png' })
    Object.defineProperty(file, 'path', {
      value: 'C:/tmp/photo.png',
    })

    await wrapper.trigger('drop', {
      dataTransfer: {
        files: [file],
      },
    })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(mocks.invoke).toHaveBeenCalledWith('show_panel')
    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'floating-file-dropped', {
      filePath: 'C:/tmp/photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })
  })

  it('emits an image action payload from Tauri window drag-drop events', async () => {
    let dragDropHandler: ((event: { payload: unknown }) => void) | null = null
    mocks.onDragDropEvent.mockImplementation((handler) => {
      dragDropHandler = handler
      return Promise.resolve(vi.fn())
    })

    mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    dragDropHandler?.({
      payload: {
        type: 'drop',
        paths: ['C:\\tmp\\photo.png'],
      },
    })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(mocks.invoke).toHaveBeenCalledWith('show_panel')
    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'floating-file-dropped', {
      filePath: 'C:\\tmp\\photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })
  })
})

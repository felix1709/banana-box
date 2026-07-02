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

  it('toggles the main panel only when the floating button is clicked', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('click')
    expect(mocks.invoke).toHaveBeenCalledWith('toggle_panel')

    expect(wrapper.text()).toBe('🍌')
  })

  it('does not toggle the main panel after dragging the floating button', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('mousedown', { button: 0, screenX: 10, screenY: 10 })
    await wrapper.trigger('mousemove', { buttons: 1, screenX: 30, screenY: 30 })
    await wrapper.trigger('click')

    expect(mocks.startDragging).toHaveBeenCalledTimes(1)
    expect(mocks.invoke).not.toHaveBeenCalledWith('toggle_panel')
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

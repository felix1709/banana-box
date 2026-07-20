import { mount } from '@vue/test-utils'
import { readFileSync } from 'node:fs'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import AnimatedBananaButton from '@/components/AnimatedBananaButton.vue'
import FloatButton from '@/components/FloatButton.vue'

const mocks = vi.hoisted(() => ({
  emitTo: vi.fn().mockResolvedValue(undefined),
  invoke: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn(),
  onDragDropEvent: vi.fn(),
  startDragging: vi.fn().mockResolvedValue(undefined),
  setSize: vi.fn().mockResolvedValue(undefined),
  setPosition: vi.fn().mockResolvedValue(undefined),
  outerPosition: vi.fn().mockResolvedValue({ x: 1200, y: 320 }),
}))

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: mocks.startDragging,
    onDragDropEvent: mocks.onDragDropEvent,
    setSize: mocks.setSize,
    setPosition: mocks.setPosition,
    outerPosition: mocks.outerPosition,
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  emitTo: mocks.emitTo,
  listen: mocks.listen,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mocks.invoke,
}))

describe('FloatButton', () => {
  let eventHandlers: Record<string, (event: { payload: unknown }) => void> = {}

  beforeEach(() => {
    mocks.emitTo.mockClear()
    mocks.invoke.mockClear()
    mocks.invoke.mockImplementation((command: string) => {
      if (command === 'get_panel_state') {
        return Promise.resolve({
          generation: 0,
          desiredVisible: false,
          actualVisible: false,
        })
      }
      return Promise.resolve(undefined)
    })
    eventHandlers = {}
    mocks.listen.mockReset()
    mocks.listen.mockImplementation((eventName, handler) => {
      eventHandlers[eventName] = handler
      return Promise.resolve(vi.fn())
    })
    mocks.onDragDropEvent.mockReset()
    mocks.onDragDropEvent.mockResolvedValue(vi.fn())
    mocks.startDragging.mockClear()
    mocks.setSize.mockClear()
    mocks.setPosition.mockClear()
    mocks.outerPosition.mockClear()
  })

  it('shows the animated banana button', () => {
    const wrapper = mount(FloatButton)

    expect(wrapper.findComponent(AnimatedBananaButton).exists()).toBe(true)
    expect(wrapper.find('.animated-banana').attributes('data-frame')).toBe('0')
  })

  it('keeps every floating window background layer transparent', () => {
    const componentSource = readFileSync('src/components/FloatButton.vue', 'utf8')
    const globalStyleSource = readFileSync('src/styles/main.css', 'utf8')

    expect(componentSource).toMatch(/\.floating-shell\s*\{[^}]*background:\s*transparent;/s)
    expect(globalStyleSource).toMatch(/#app\s*\{[^}]*background:\s*transparent;/s)
  })

  it('does not draw a shadow backdrop behind the floating reminder card', () => {
    const componentSource = readFileSync('src/components/FloatButton.vue', 'utf8')

    expect(componentSource).toMatch(/\.floating-reminder\s*\{[^}]*box-shadow:\s*none;/s)
  })

  it('toggles the main panel only when the floating button is clicked', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('click')

    expect(mocks.invoke).toHaveBeenCalledWith('toggle_panel', {})
    expect(wrapper.findComponent(AnimatedBananaButton).exists()).toBe(true)
  })

  it('does not toggle the main panel after dragging the floating button', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('mousedown', { button: 0, screenX: 10, screenY: 10 })
    await wrapper.trigger('mousemove', { buttons: 1, screenX: 30, screenY: 30 })
    await wrapper.trigger('click')

    expect(mocks.startDragging).toHaveBeenCalledTimes(1)
    expect(mocks.invoke).not.toHaveBeenCalledWith('toggle_panel', {})
  })

  it('opens the banana from Rust panel state and acknowledges the reveal frame', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['panel-target-changed']?.({
      payload: {
        generation: 7,
        targetVisible: true,
        reason: 'banana',
        revealAtFrame: 6,
      },
    })
    await wrapper.vm.$nextTick()
    wrapper.findComponent(AnimatedBananaButton).vm.$emit('frame', 6)

    expect(mocks.invoke).toHaveBeenCalledWith('ack_panel_reveal', {
      generation: 7,
      frame: 6,
    })
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

    expect(mocks.invoke).toHaveBeenCalledWith('show_panel', {})
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

    expect(mocks.invoke).toHaveBeenCalledWith('show_panel', {})
    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'floating-file-dropped', {
      filePath: 'C:\\tmp\\photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })
  })

  it('shows a reminder dialog inside the floating window when a daily reminder arrives', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-reminder']?.({
      payload: {
        taskId: 't1',
        title: 'Shot refinement',
        body: 'Check delivery notes',
        time: '10:01',
        localDate: '2026-07-12',
      },
    })
    await wrapper.vm.$nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.get('[data-floating-reminder]').text()).toContain('Shot refinement')
    expect(wrapper.get('[data-floating-reminder]').text()).toContain('Check delivery notes')
    expect(mocks.setSize).toHaveBeenCalled()
    expect(mocks.setPosition).toHaveBeenCalled()
  })

  it('lets users snooze a floating reminder and closes the reminder window', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-reminder']?.({
      payload: {
        taskId: 't1',
        title: 'Shot refinement',
        body: 'Check delivery notes',
        time: '10:01',
        localDate: '2026-07-12',
      },
    })
    await wrapper.vm.$nextTick()

    await wrapper.get('[data-action="snooze-reminder-10"]').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'daily-task-reminder-snooze', {
      taskId: 't1',
      localDate: '2026-07-12',
      minutes: 10,
    })
    expect(wrapper.find('[data-floating-reminder]').exists()).toBe(false)
    expect(mocks.setSize).toHaveBeenLastCalledWith(expect.objectContaining({ width: 64, height: 64 }))
  })

  it('shows the daily task review card in the floating window', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-review-start']?.({
      payload: reviewPayload(),
    })
    await wrapper.vm.$nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.get('[data-floating-daily-review]').text()).toContain('每日任务确认')
    expect(wrapper.get('[data-floating-daily-review]').text()).toContain('1 / 2')
    expect(wrapper.get('[data-floating-daily-review]').text()).toContain('Shot refinement')
    expect(wrapper.get('[data-floating-daily-review]').text()).toContain('45%')
    expect(mocks.setSize).toHaveBeenCalled()
    expect(mocks.setPosition).toHaveBeenCalled()
  })

  it('emits complete and delay actions from the daily task review card', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-review-start']?.({
      payload: reviewPayload(),
    })
    await wrapper.vm.$nextTick()

    await wrapper.get('[data-action="daily-review-complete"]').trigger('click')
    await wrapper.get('[data-action="daily-review-delay"]').trigger('click')

    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'daily-task-review-complete-task', {
      sessionId: 'session-1',
      taskId: 'task-1',
    })
    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'daily-task-review-delay-task', {
      sessionId: 'session-1',
      taskId: 'task-1',
    })
  })

  it('shows the daily report copy action after review completion', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-review-complete']?.({
      payload: { sessionId: 'session-1', localDate: '2026-07-20' },
    })
    await wrapper.vm.$nextTick()

    expect(wrapper.get('[data-floating-daily-review]').text()).toContain('全部确认完成')

    await wrapper.get('[data-action="daily-review-copy-report"]').trigger('click')

    expect(mocks.emitTo).toHaveBeenCalledWith('main', 'daily-task-review-copy-report', {
      sessionId: 'session-1',
    })
  })

  it('closes the daily task review card when the main window confirms close', async () => {
    const wrapper = mount(FloatButton)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    eventHandlers['daily-task-review-start']?.({
      payload: reviewPayload(),
    })
    await wrapper.vm.$nextTick()

    eventHandlers['daily-task-review-close']?.({
      payload: { sessionId: 'session-1' },
    })
    await wrapper.vm.$nextTick()

    expect(wrapper.find('[data-floating-daily-review]').exists()).toBe(false)
    expect(mocks.setSize).toHaveBeenLastCalledWith(expect.objectContaining({ width: 64, height: 64 }))
  })
})

function reviewPayload() {
  return {
    sessionId: 'session-1',
    localDate: '2026-07-20',
    index: 0,
    total: 2,
    task: {
      taskId: 'task-1',
      code: 'L36',
      projectId: null,
      title: 'Shot refinement',
      progress: 45,
      note: 'Check final color',
      investedMinutes: 20,
      reminderTime: '',
      reminderContent: '',
    },
  }
}

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from '@/App.vue'
import PromptCard from '@/components/PromptCard.vue'
import { useLibraryStore } from '@/stores/library'
import { useCloudSessionStore } from '@/stores/cloudSession'
import { useAuthStore } from '@/stores/auth'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useUiStore } from '@/stores/ui'
import * as ipc from '@/lib/ipc'

let eventHandlers: Record<string, (event: { payload: unknown }) => void> = {}
const coreApi = vi.hoisted(() => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}))
const eventApi = vi.hoisted(() => ({
  emitTo: vi.fn().mockResolvedValue(undefined),
}))
const productionIpc = vi.hoisted(() => ({
  listProjects: vi.fn().mockResolvedValue([]),
  loadDailyTaskDay: vi.fn(),
  createDailyTask: vi.fn(),
  updateDailyTask: vi.fn(),
  getDailyReport: vi.fn(),
}))
const windowApi = vi.hoisted(() => ({
  startResizeDragging: vi.fn().mockResolvedValue(undefined),
  setFullscreen: vi.fn().mockResolvedValue(undefined),
  isFullscreen: vi.fn().mockResolvedValue(false),
  setSize: vi.fn().mockResolvedValue(undefined),
  center: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@tauri-apps/api/event', () => ({
  emitTo: eventApi.emitTo,
  listen: vi.fn((eventName: string, handler: (event: { payload: unknown }) => void) => {
    eventHandlers[eventName] = handler
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
    },
  }),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  loadCloudConfig: vi.fn().mockResolvedValue({
    supabaseUrl: '',
    hasAnonKey: false,
    cloudEnabled: false,
    updatedAt: null,
  }),
  loadCloudRuntimeConfig: vi.fn().mockResolvedValue({
    supabaseUrl: '',
    anonKey: '',
    cloudEnabled: false,
  }),
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
}))

vi.mock('@/lib/productionIpc', () => productionIpc)

describe('App', () => {
  beforeEach(() => {
    vi.useRealTimers()
    setActivePinia(createPinia())
    eventHandlers = {}
    vi.clearAllMocks()
    eventApi.emitTo.mockClear()
    productionIpc.listProjects.mockResolvedValue([])
    productionIpc.loadDailyTaskDay.mockReset()
    productionIpc.createDailyTask.mockReset()
    productionIpc.updateDailyTask.mockReset()
    productionIpc.getDailyReport.mockReset()
    windowApi.isFullscreen.mockResolvedValue(false)
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

  it('grants the window permissions required by fullscreen and restore-size controls', () => {
    const capability = JSON.parse(
      readFileSync(resolve(process.cwd(), 'src-tauri/capabilities/default.json'), 'utf8'),
    ) as { permissions: string[] }

    expect(capability.permissions).toEqual(
      expect.arrayContaining([
        'core:window:allow-set-fullscreen',
        'core:window:allow-set-size',
        'core:window:allow-center',
      ]),
    )
  })

  it('toggles the main window fullscreen state from the icon control', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const fullscreenButton = wrapper.findAll('.window-command')[0]
    await fullscreenButton.trigger('click')

    expect(windowApi.setFullscreen).toHaveBeenCalledWith(true)
    expect(fullscreenButton.attributes('aria-label')).toBe('退出全屏')
  })

  it('falls back to browser fullscreen when the Tauri window API is unavailable', async () => {
    const originalRequestFullscreen = document.documentElement.requestFullscreen
    const requestFullscreen = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(document.documentElement, 'requestFullscreen', {
      configurable: true,
      value: requestFullscreen,
    })
    windowApi.setFullscreen.mockRejectedValueOnce(new Error('Tauri window API unavailable'))

    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.findAll('.window-command')[0].trigger('click')

    expect(requestFullscreen).toHaveBeenCalledTimes(1)

    Object.defineProperty(document.documentElement, 'requestFullscreen', {
      configurable: true,
      value: originalRequestFullscreen,
    })
  })

  it('exits browser fullscreen from the restore-size control when Tauri is unavailable', async () => {
    const originalExitFullscreen = document.exitFullscreen
    const originalFullscreenElement = Object.getOwnPropertyDescriptor(document, 'fullscreenElement')
    const exitFullscreen = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(document, 'fullscreenElement', {
      configurable: true,
      value: document.documentElement,
    })
    Object.defineProperty(document, 'exitFullscreen', {
      configurable: true,
      value: exitFullscreen,
    })
    windowApi.setFullscreen.mockRejectedValueOnce(new Error('Tauri window API unavailable'))

    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.findAll('.window-command')[1].trigger('click')

    expect(exitFullscreen).toHaveBeenCalledTimes(1)

    Object.defineProperty(document, 'exitFullscreen', {
      configurable: true,
      value: originalExitFullscreen,
    })
    if (originalFullscreenElement) {
      Object.defineProperty(document, 'fullscreenElement', originalFullscreenElement)
    } else {
      Reflect.deleteProperty(document, 'fullscreenElement')
    }
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

    eventHandlers['floating-file-dropped']?.({
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

  it('sends due daily task reminders to the floating window', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 12, 10, 0, 0))
    const daily = useDailyTasksStore()
    daily.selectedDate = '2026-07-12'
    daily.day = {
      id: 'd1',
      localDate: '2026-07-12',
      settledAt: null,
      reportSnapshot: null,
      groups: [{
        id: 'g1',
        code: 'L36',
        projectId: null,
        position: 0,
        tasks: [{
          id: 't1',
          title: 'Shot refinement',
          progress: 45,
          note: '',
          investedMinutes: 0,
          reminderTime: '10:01',
          reminderContent: 'Check delivery notes',
          position: 0,
          sourceTaskId: null,
          sourceSnapshotHash: null,
          createdAt: '2026-07-12T08:00:00Z',
          updatedAt: '2026-07-12T08:00:00Z',
        }],
      }],
    }

    const wrapper = mount(App)
    await vi.advanceTimersByTimeAsync(0)
    coreApi.invoke.mockClear()

    await vi.advanceTimersByTimeAsync(60_000)
    await wrapper.vm.$nextTick()

    expect(eventApi.emitTo).toHaveBeenCalledWith('floatbtn', 'daily-task-reminder', {
      taskId: 't1',
      title: 'Shot refinement',
      body: 'Check delivery notes',
      time: '10:01',
      localDate: '2026-07-12',
    })
    expect(wrapper.find('[data-reminder-dialog]').exists()).toBe(false)
  })

  it('snoozes a due daily task reminder by updating the task reminder time', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 12, 10, 1, 0))
    const daily = useDailyTasksStore()
    const update = vi.spyOn(daily, 'update').mockResolvedValue(undefined)
    daily.selectedDate = '2026-07-12'
    daily.day = {
      id: 'd1',
      localDate: '2026-07-12',
      settledAt: null,
      reportSnapshot: null,
      groups: [{
        id: 'g1',
        code: 'L36',
        projectId: null,
        position: 0,
        tasks: [{
          id: 't1',
          title: 'Shot refinement',
          progress: 45,
          note: 'Keep color pass tight',
          investedMinutes: 30,
          reminderTime: '10:01',
          reminderContent: 'Check delivery notes',
          position: 0,
          sourceTaskId: null,
          sourceSnapshotHash: null,
          createdAt: '2026-07-12T08:00:00Z',
          updatedAt: '2026-07-12T08:00:00Z',
        }],
      }],
    }

    mount(App)
    await vi.advanceTimersByTimeAsync(0)

    eventHandlers['daily-task-reminder-snooze']?.({
      payload: {
        taskId: 't1',
        localDate: '2026-07-12',
        minutes: 10,
      },
    })
    await vi.advanceTimersByTimeAsync(0)

    expect(update).toHaveBeenCalledWith({
      taskId: 't1',
      title: 'Shot refinement',
      progress: 45,
      note: 'Keep color pass tight',
      investedMinutes: 30,
      reminderTime: '10:11',
      reminderContent: 'Check delivery notes',
    })
  })

  it('starts a workday daily task review at 18:25 while the app is running', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 24, 0))
    productionIpc.loadDailyTaskDay.mockResolvedValue(reviewDay())

    mount(App)
    await vi.advanceTimersByTimeAsync(0)
    eventApi.emitTo.mockClear()

    await vi.advanceTimersByTimeAsync(60_000)

    expect(productionIpc.loadDailyTaskDay).toHaveBeenCalledWith('2026-07-20')
    expect(eventApi.emitTo).toHaveBeenCalledWith('floatbtn', 'daily-task-review-start', {
      sessionId: expect.any(String),
      localDate: '2026-07-20',
      index: 0,
      total: 2,
      task: expect.objectContaining({
        taskId: 'task-1',
        title: 'Already done',
        progress: 100,
      }),
    })
  })

  it('does not catch up when mounted after 18:25', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 26, 0))
    productionIpc.loadDailyTaskDay.mockResolvedValue(reviewDay())

    mount(App)
    await vi.advanceTimersByTimeAsync(1_000)

    expect(productionIpc.loadDailyTaskDay).not.toHaveBeenCalled()
    expect(eventApi.emitTo).not.toHaveBeenCalledWith(
      'floatbtn',
      'daily-task-review-start',
      expect.anything(),
    )
  })

  it('does not start the workday review on weekends', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 25, 18, 24, 0))
    productionIpc.loadDailyTaskDay.mockResolvedValue(reviewDay())

    mount(App)
    await vi.advanceTimersByTimeAsync(60_000)

    expect(productionIpc.loadDailyTaskDay).not.toHaveBeenCalled()
  })

  it('marks the current review task complete and advances to the next task', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 24, 0))
    productionIpc.loadDailyTaskDay.mockResolvedValue(reviewDay())
    productionIpc.updateDailyTask.mockResolvedValue(reviewDay())

    mount(App)
    await vi.advanceTimersByTimeAsync(60_000)
    const startPayload = eventApi.emitTo.mock.calls.find(
      (call) => call[1] === 'daily-task-review-start',
    )?.[2] as { sessionId: string }
    eventApi.emitTo.mockClear()

    eventHandlers['daily-task-review-complete-task']?.({
      payload: { sessionId: startPayload.sessionId, taskId: 'task-1' },
    })
    await vi.advanceTimersByTimeAsync(0)

    expect(productionIpc.updateDailyTask).toHaveBeenCalledWith({
      taskId: 'task-1',
      title: 'Already done',
      progress: 100,
      note: 'Completed before review',
      investedMinutes: 20,
      reminderTime: '',
      reminderContent: '',
    })
    expect(eventApi.emitTo).toHaveBeenCalledWith('floatbtn', 'daily-task-review-update', {
      sessionId: startPayload.sessionId,
      localDate: '2026-07-20',
      index: 1,
      total: 2,
      task: expect.objectContaining({ taskId: 'task-2' }),
    })
  })

  it('delays a review task to tomorrow with the same progress and metadata', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 24, 0))
    productionIpc.loadDailyTaskDay
      .mockResolvedValueOnce(reviewDay())
      .mockResolvedValueOnce(emptyDay('2026-07-21'))
    productionIpc.createDailyTask.mockResolvedValue(emptyDay('2026-07-21'))

    mount(App)
    await vi.advanceTimersByTimeAsync(60_000)
    const startPayload = eventApi.emitTo.mock.calls.find(
      (call) => call[1] === 'daily-task-review-start',
    )?.[2] as { sessionId: string }

    eventHandlers['daily-task-review-delay-task']?.({
      payload: { sessionId: startPayload.sessionId, taskId: 'task-1' },
    })
    await vi.advanceTimersByTimeAsync(0)

    expect(productionIpc.createDailyTask).toHaveBeenCalledWith({
      localDate: '2026-07-21',
      code: 'L36',
      projectId: null,
      title: 'Already done',
      progress: 100,
      note: 'Completed before review',
      investedMinutes: 20,
      reminderTime: '',
      reminderContent: '',
    })
  })

  it('does not duplicate a delayed task when tomorrow has the same title', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 24, 0))
    productionIpc.loadDailyTaskDay
      .mockResolvedValueOnce(reviewDay())
      .mockResolvedValueOnce(reviewDay('2026-07-21'))

    mount(App)
    await vi.advanceTimersByTimeAsync(60_000)
    const startPayload = eventApi.emitTo.mock.calls.find(
      (call) => call[1] === 'daily-task-review-start',
    )?.[2] as { sessionId: string }

    eventHandlers['daily-task-review-delay-task']?.({
      payload: { sessionId: startPayload.sessionId, taskId: 'task-1' },
    })
    await vi.advanceTimersByTimeAsync(0)

    expect(productionIpc.createDailyTask).not.toHaveBeenCalled()
  })

  it('copies the daily report after every review task is processed', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date(2026, 6, 20, 18, 24, 0))
    productionIpc.loadDailyTaskDay.mockResolvedValue(oneTaskReviewDay())
    productionIpc.updateDailyTask.mockResolvedValue(oneTaskReviewDay())
    productionIpc.getDailyReport.mockResolvedValue({ text: '@日报\n#L36', taskCount: 1 })

    mount(App)
    await vi.advanceTimersByTimeAsync(60_000)
    const startPayload = eventApi.emitTo.mock.calls.find(
      (call) => call[1] === 'daily-task-review-start',
    )?.[2] as { sessionId: string }

    eventHandlers['daily-task-review-complete-task']?.({
      payload: { sessionId: startPayload.sessionId, taskId: 'task-1' },
    })
    await vi.advanceTimersByTimeAsync(0)
    eventHandlers['daily-task-review-copy-report']?.({
      payload: { sessionId: startPayload.sessionId },
    })
    await vi.advanceTimersByTimeAsync(0)

    expect(productionIpc.getDailyReport).toHaveBeenCalledWith('2026-07-20')
    expect(ipc.copyToClipboard).toHaveBeenCalledWith('@日报\n#L36')
    expect(eventApi.emitTo).toHaveBeenCalledWith('floatbtn', 'daily-task-review-close', {
      sessionId: startPayload.sessionId,
    })
  })

  it('does not render the old separate category pane in the prompt library', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('.category-pane').exists()).toBe(false)
    expect(wrapper.find('.sidebar-category-list').exists()).toBe(false)

    await wrapper.find('[data-tool="prompts"]').trigger('click')

    expect(wrapper.find('.sidebar-category-list').exists()).toBe(true)
  })

  it('refreshes auth when cloud settings become ready after the app has mounted', async () => {
    const wrapper = mount(App)
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    const auth = useAuthStore()
    const initialize = vi.spyOn(auth, 'initialize').mockResolvedValue(undefined)

    useCloudSessionStore().config = {
      supabaseUrl: 'https://example.supabase.co',
      hasAnonKey: true,
      cloudEnabled: true,
      updatedAt: '2026-07-13T15:50:00Z',
    }
    await wrapper.vm.$nextTick()
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(initialize).toHaveBeenCalled()
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

function reviewDay(localDate = '2026-07-20') {
  return {
    id: `day-${localDate}`,
    localDate,
    settledAt: null,
    reportSnapshot: null,
    groups: [{
      id: 'group-1',
      code: 'L36',
      projectId: null,
      position: 0,
      tasks: [
        reviewTask('task-1', 'Already done', 100, 'Completed before review', 0),
        reviewTask('task-2', 'Continue edit', 45, 'Needs another pass', 1),
      ],
    }],
  }
}

function oneTaskReviewDay(localDate = '2026-07-20') {
  const currentDay = reviewDay(localDate)
  currentDay.groups[0].tasks = [
    reviewTask('task-1', 'Already done', 100, 'Completed before review', 0),
  ]
  return currentDay
}

function emptyDay(localDate: string) {
  return {
    id: `day-${localDate}`,
    localDate,
    settledAt: null,
    reportSnapshot: null,
    groups: [],
  }
}

function reviewTask(
  id: string,
  title: string,
  progress: number,
  note: string,
  position: number,
) {
  return {
    id,
    title,
    progress,
    note,
    investedMinutes: 20,
    reminderTime: '',
    reminderContent: '',
    position,
    sourceTaskId: null,
    sourceSnapshotHash: null,
    createdAt: '2026-07-20T08:00:00Z',
    updatedAt: '2026-07-20T08:00:00Z',
  }
}

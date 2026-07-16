import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { readFileSync } from 'node:fs'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import DailyTasksPage from '@/components/daily/DailyTasksPage.vue'
import type { DailyTaskDay } from '@/domain/production'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useUiStore } from '@/stores/ui'

const getDailyReport = vi.hoisted(() => vi.fn())
const copyToClipboard = vi.hoisted(() => vi.fn())

vi.mock('@/lib/productionIpc', () => ({ getDailyReport }))
vi.mock('@/lib/ipc', () => ({ copyToClipboard }))

describe('DailyTasksPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    getDailyReport.mockReset()
    copyToClipboard.mockReset()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('loads the selected date and lets the user move one day at a time', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-12T08:30:00+08:00'))
    const store = useDailyTasksStore()
    store.selectedDate = '2026-07-12'
    const selectDate = vi.spyOn(store, 'selectDate').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    expect(selectDate).toHaveBeenCalledWith('2026-07-12')
    expect((wrapper.get('[data-field="daily-date"]').element as HTMLInputElement).value).toBe('2026-07-12')

    await wrapper.get('[data-action="previous-day"]').trigger('click')
    expect(selectDate).toHaveBeenLastCalledWith('2026-07-11')

    await wrapper.get('[data-action="next-day"]').trigger('click')
    expect(selectDate).toHaveBeenLastCalledWith('2026-07-13')
  })

  it('opens on the current system date even when the stored date is stale', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-16T08:30:00+08:00'))
    const store = useDailyTasksStore()
    store.selectedDate = '2026-07-15'
    const selectDate = vi.spyOn(store, 'selectDate').mockImplementation(async (localDate: string) => {
      store.selectedDate = localDate
    })
    const wrapper = mount(DailyTasksPage)
    await wrapper.vm.$nextTick()

    expect(selectDate).toHaveBeenCalledWith('2026-07-16')
    expect((wrapper.get('[data-field="daily-date"]').element as HTMLInputElement).value).toBe('2026-07-16')
  })

  it('switches to the new system date while the daily page stays open across midnight', async () => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-15T23:59:30+08:00'))
    const store = useDailyTasksStore()
    const selectDate = vi.spyOn(store, 'selectDate').mockImplementation(async (localDate: string) => {
      store.selectedDate = localDate
    })
    mount(DailyTasksPage)

    expect(selectDate).toHaveBeenCalledWith('2026-07-15')

    vi.setSystemTime(new Date('2026-07-16T00:00:31+08:00'))
    await vi.advanceTimersByTimeAsync(60_000)

    expect(selectDate).toHaveBeenCalledWith('2026-07-16')
  })

  it('opens a secondary create popover and creates a task with the entered code, title, progress, and note', async () => {
    const store = useDailyTasksStore()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const create = vi.spyOn(store, 'create').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    expect(wrapper.find('.daily-create').exists()).toBe(false)

    await wrapper.get('[data-action="open-create-daily-task"]').trigger('click')
    expect(document.body.querySelector('.daily-create-popover')).toBeTruthy()

    ;(document.body.querySelector('[data-field="new-task-code"]') as HTMLInputElement).value = 'L36'
    document.body.querySelector('[data-field="new-task-code"]')?.dispatchEvent(new Event('input', { bubbles: true }))
    ;(document.body.querySelector('[data-field="new-task-title"]') as HTMLInputElement).value = 'Shot refinement'
    document.body.querySelector('[data-field="new-task-title"]')?.dispatchEvent(new Event('input', { bubbles: true }))
    ;(document.body.querySelector('[data-field="new-task-progress"]') as HTMLInputElement).value = '45'
    document.body.querySelector('[data-field="new-task-progress"]')?.dispatchEvent(new Event('input', { bubbles: true }))
    ;(document.body.querySelector('[data-field="new-task-note"]') as HTMLTextAreaElement).value = 'Transition pass'
    document.body.querySelector('[data-field="new-task-note"]')?.dispatchEvent(new Event('input', { bubbles: true }))
    ;(document.body.querySelector('[data-action="create-daily-task"]') as HTMLButtonElement).click()
    await Promise.resolve()
    await wrapper.vm.$nextTick()

    expect(create).toHaveBeenCalledWith({
      code: 'L36',
      title: 'Shot refinement',
      progress: 45,
      note: 'Transition pass',
      investedMinutes: 0,
    })
    expect(wrapper.find('[data-field="new-task-minutes"]').exists()).toBe(false)
    expect(document.body.querySelector('.daily-create-popover')).toBeFalsy()
  })

  it('auto-saves progress changes, keeps delete action, and hides empty notes', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    store.day.groups[0].tasks[0].note = ''
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const update = vi.spyOn(store, 'update').mockResolvedValue()
    const remove = vi.spyOn(store, 'remove').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    expect(wrapper.get('[data-task-group="L36"]').text()).toContain('L36')
    expect((wrapper.get('[data-task-id="t1"] [data-field="task-title"]').element as HTMLInputElement).value).toBe('Shot refinement')

    const progress = wrapper.get('[data-task-id="t1"] [data-field="task-progress"]')
    expect(progress.attributes('type')).toBe('range')
    expect(wrapper.get('[data-task-id="t1"] .task-card-actions').exists()).toBe(true)
    expect(wrapper.get('[data-task-id="t1"] .task-card-actions').find('[data-action="task-reminder"]').exists()).toBe(true)
    expect(wrapper.get('[data-task-id="t1"] .task-card-actions').find('[data-action="delete-task"]').exists()).toBe(true)
    const source = readFileSync('src/components/daily/DailyTasksPage.vue', 'utf8')
    expect(source).toMatch(/\.daily-task\s*\{[^}]*position:relative;/s)
    expect(source).toMatch(/\.task-card-actions\s*\{[^}]*position:absolute;[^}]*top:10px;[^}]*right:10px;/s)
    expect(wrapper.find('[data-task-id="t1"] [data-field="task-note"]').exists()).toBe(false)
    expect(wrapper.find('[data-task-id="t1"] [data-action="save-task"]').exists()).toBe(false)

    await progress.setValue(80)
    expect(wrapper.get('[data-task-id="t1"] [data-progress-value]').text()).toBe('80%')
    expect(update).toHaveBeenCalledWith({
      taskId: 't1',
      title: 'Shot refinement',
      progress: 80,
      note: '',
      investedMinutes: 90,
      reminderTime: '',
      reminderContent: '',
    })

    await wrapper.get('[data-task-id="t1"] [data-action="delete-task"]').trigger('click')
    expect(remove).toHaveBeenCalledWith('t1')
  })

  it('opens a reminder popover from the alarm icon and saves reminder details', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const update = vi.spyOn(store, 'update').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-task-id="t1"] [data-action="task-reminder"]').trigger('click')
    expect(document.body.querySelector('.task-reminder-popover')).toBeTruthy()

    const time = document.body.querySelector('[data-field="task-reminder-time"]') as HTMLInputElement
    const content = document.body.querySelector('[data-field="task-reminder-content"]') as HTMLTextAreaElement
    time.value = '18:30'
    time.dispatchEvent(new Event('input', { bubbles: true }))
    content.value = 'Check delivery notes'
    content.dispatchEvent(new Event('input', { bubbles: true }))

    const save = document.body.querySelector('[data-action="save-task-reminder"]') as HTMLButtonElement
    save.click()
    await Promise.resolve()

    expect(update).toHaveBeenCalledWith({
      taskId: 't1',
      title: 'Shot refinement',
      progress: 45,
      note: 'Transition pass',
      investedMinutes: 90,
      reminderTime: '18:30',
      reminderContent: 'Check delivery notes',
    })
  })

  it('closes the reminder popover when clicking outside it', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-task-id="t1"] [data-action="task-reminder"]').trigger('click')
    expect(document.body.querySelector('.task-reminder-popover')).toBeTruthy()

    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await wrapper.vm.$nextTick()

    expect(document.body.querySelector('.task-reminder-popover')).toBeFalsy()
  })

  it('keeps the reminder popover inside the visible window near the bottom edge', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue({
      x: 260,
      y: 130,
      top: 130,
      right: 290,
      bottom: 160,
      left: 260,
      width: 30,
      height: 30,
      toJSON: () => ({}),
    } as DOMRect)
    const originalInnerHeight = window.innerHeight
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 190 })
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-task-id="t1"] [data-action="task-reminder"]').trigger('click')
    const popover = document.body.querySelector('.task-reminder-popover') as HTMLElement

    expect(Number.parseFloat(popover.style.top)).toBeLessThan(130)

    rectSpy.mockRestore()
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: originalInnerHeight })
  })

  it('opens the task editor by double-clicking a task card and saves edits', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const update = vi.spyOn(store, 'update').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-task-id="t1"]').trigger('dblclick')
    expect(wrapper.find('.task-editor-dialog').exists()).toBe(true)

    await wrapper.get('[data-field="task-editor-title"]').setValue('Updated task')
    await wrapper.get('[data-action="save-task-editor"]').trigger('click')

    expect(update).toHaveBeenCalledWith({
      taskId: 't1',
      title: 'Updated task',
      progress: 45,
      note: 'Transition pass',
      investedMinutes: 90,
      reminderTime: '',
      reminderContent: '',
    })
  })

  it('shows a read-only settlement notice for settled days', async () => {
    const store = useDailyTasksStore()
    store.day = { ...day(), settledAt: '2026-07-12T10:00:00Z' }
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    expect(wrapper.get('[data-settled-notice]').exists()).toBe(true)
    expect(wrapper.find('[data-action="create-daily-task"]').exists()).toBe(false)
    expect(wrapper.find('[data-action="save-task"]').exists()).toBe(false)
  })

  it('copies the complete Markdown daily report for the selected date', async () => {
    const store = useDailyTasksStore()
    store.selectedDate = '2026-07-12'
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    getDailyReport.mockResolvedValue({
      text: '@日报\n#L36\n1.【L36】【Shot refinement】【45%】',
      taskCount: 1,
    })
    copyToClipboard.mockResolvedValue(undefined)
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-action="copy-daily-report"]').trigger('click')

    expect(getDailyReport).toHaveBeenCalledWith('2026-07-12')
    expect(copyToClipboard).toHaveBeenCalledWith(
      '@日报\n#L36\n1.【L36】【Shot refinement】【45%】',
    )
  })

  it('restores the report copy icon after 1.2 seconds', async () => {
    vi.useFakeTimers()
    const store = useDailyTasksStore()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    getDailyReport.mockResolvedValue({ text: '@日报', taskCount: 0 })
    copyToClipboard.mockResolvedValue(undefined)
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-action="copy-daily-report"]').trigger('click')
    await Promise.resolve()
    expect(wrapper.get('[data-action="copy-daily-report"]').attributes('data-copy-state')).toBe('copied')

    await vi.advanceTimersByTimeAsync(1200)
    expect(wrapper.get('[data-action="copy-daily-report"]').attributes('data-copy-state')).toBe('ready')
  })

  it('keeps the report copy icon and shows a toast when copying fails', async () => {
    const store = useDailyTasksStore()
    const ui = useUiStore()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    getDailyReport.mockResolvedValue({ text: '@日报', taskCount: 0 })
    copyToClipboard.mockRejectedValue(new Error('clipboard unavailable'))
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-action="copy-daily-report"]').trigger('click')
    await Promise.resolve()

    expect(wrapper.get('[data-action="copy-daily-report"]').attributes('data-copy-state')).toBe('ready')
    expect(ui.toast).toBe('复制日报失败')
  })
})

function day(): DailyTaskDay {
  return {
    id: 'd1',
    localDate: '2026-07-12',
    settledAt: null,
    reportSnapshot: null,
    groups: [
      {
        id: 'g1',
        code: 'L36',
        projectId: null,
        position: 0,
        tasks: [
          {
            id: 't1',
            title: 'Shot refinement',
            progress: 45,
            note: 'Transition pass',
            investedMinutes: 90,
            reminderTime: '',
            reminderContent: '',
            position: 0,
            sourceTaskId: null,
            sourceSnapshotHash: null,
            createdAt: '2026-07-12T08:00:00Z',
            updatedAt: '2026-07-12T08:00:00Z',
          },
        ],
      },
    ],
  }
}

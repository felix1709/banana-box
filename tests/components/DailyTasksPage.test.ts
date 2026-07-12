import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DailyTasksPage from '@/components/daily/DailyTasksPage.vue'
import type { DailyTaskDay } from '@/domain/production'
import { useDailyTasksStore } from '@/stores/dailyTasks'

describe('DailyTasksPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('loads the selected date and lets the user move one day at a time', async () => {
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

  it('creates a task with the entered code, title, progress, note, and effort', async () => {
    const store = useDailyTasksStore()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const create = vi.spyOn(store, 'create').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    await wrapper.get('[data-field="new-task-code"]').setValue('L36')
    await wrapper.get('[data-field="new-task-title"]').setValue('Shot refinement')
    await wrapper.get('[data-field="new-task-progress"]').setValue(45)
    await wrapper.get('[data-field="new-task-note"]').setValue('Transition pass')
    await wrapper.get('[data-field="new-task-minutes"]').setValue(90)
    await wrapper.get('[data-action="create-daily-task"]').trigger('click')

    expect(create).toHaveBeenCalledWith({
      code: 'L36',
      title: 'Shot refinement',
      progress: 45,
      note: 'Transition pass',
      investedMinutes: 90,
    })
  })

  it('renders groups and saves or deletes an edited task with explicit controls', async () => {
    const store = useDailyTasksStore()
    store.day = day()
    vi.spyOn(store, 'selectDate').mockResolvedValue()
    const update = vi.spyOn(store, 'update').mockResolvedValue()
    const remove = vi.spyOn(store, 'remove').mockResolvedValue()
    const wrapper = mount(DailyTasksPage)

    expect(wrapper.get('[data-task-group="L36"]').text()).toContain('L36')
    expect((wrapper.get('[data-task-id="t1"] [data-field="task-title"]').element as HTMLInputElement).value).toBe('Shot refinement')

    await wrapper.get('[data-task-id="t1"] [data-field="task-progress"]').setValue(80)
    await wrapper.get('[data-task-id="t1"] [data-action="save-task"]').trigger('click')
    expect(update).toHaveBeenCalledWith({
      taskId: 't1',
      title: 'Shot refinement',
      progress: 80,
      note: 'Transition pass',
      investedMinutes: 90,
    })

    await wrapper.get('[data-task-id="t1"] [data-action="delete-task"]').trigger('click')
    expect(remove).toHaveBeenCalledWith('t1')
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

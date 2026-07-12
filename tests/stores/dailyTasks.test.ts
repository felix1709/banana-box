import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useDailyTasksStore } from '@/stores/dailyTasks'

const loadDailyTaskDay = vi.hoisted(() => vi.fn())
const createDailyTask = vi.hoisted(() => vi.fn())

vi.mock('@/lib/productionIpc', () => ({
  createDailyTask,
  deleteDailyTask: vi.fn(),
  loadDailyTaskDay,
  reorderDailyGroups: vi.fn(),
  reorderDailyTasks: vi.fn(),
  updateDailyTask: vi.fn(),
}))

describe('daily tasks store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    loadDailyTaskDay.mockReset()
    createDailyTask.mockReset()
  })

  it('loads a selected date and replaces it after a task is created', async () => {
    const store = useDailyTasksStore()
    loadDailyTaskDay.mockResolvedValue(day('2026-07-11'))
    createDailyTask.mockResolvedValue(day('2026-07-11', 1))

    await store.selectDate('2026-07-11')
    await store.create({ code: 'L36', title: '三丽鸥跟进', progress: 50, note: '', investedMinutes: 0 })

    expect(loadDailyTaskDay).toHaveBeenCalledWith('2026-07-11')
    expect(createDailyTask).toHaveBeenCalledWith(expect.objectContaining({ localDate: '2026-07-11', code: 'L36' }))
    expect(store.day?.groups[0].tasks).toHaveLength(1)
  })
})

function day(localDate: string, tasks = 0) {
  return {
    id: 'd1', localDate, settledAt: null, reportSnapshot: null,
    groups: tasks ? [{ id: 'g1', code: 'L36', projectId: null, position: 0, tasks: [{ id: 't1', title: '三丽鸥跟进', progress: 50, note: '', investedMinutes: 0, position: 0, sourceTaskId: null, sourceSnapshotHash: null, createdAt: '', updatedAt: '' }] }] : [],
  }
}

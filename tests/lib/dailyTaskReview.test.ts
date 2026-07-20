import { describe, expect, it } from 'vitest'
import type { DailyTaskDay } from '@/domain/production'
import {
  addLocalDays,
  buildDailyTaskReviewItems,
  hasExactTitle,
  isWorkday,
  nextWorkdayReviewDelay,
} from '@/lib/dailyTaskReview'

describe('daily task review helpers', () => {
  it('identifies Monday through Friday as workdays', () => {
    expect(isWorkday(new Date(2026, 6, 20, 18, 25))).toBe(true)
    expect(isWorkday(new Date(2026, 6, 24, 18, 25))).toBe(true)
    expect(isWorkday(new Date(2026, 6, 25, 18, 25))).toBe(false)
    expect(isWorkday(new Date(2026, 6, 26, 18, 25))).toBe(false)
  })

  it('schedules today when the app is running before 18:25 on a workday', () => {
    const now = new Date(2026, 6, 20, 18, 24, 0)

    expect(nextWorkdayReviewDelay(now)).toEqual({
      localDate: '2026-07-20',
      delayMs: 60_000,
    })
  })

  it('does not catch up when the app starts after 18:25', () => {
    const now = new Date(2026, 6, 20, 18, 26, 0)

    expect(nextWorkdayReviewDelay(now)).toEqual({
      localDate: '2026-07-21',
      delayMs: 86_340_000,
    })
  })

  it('skips weekends and schedules Monday 18:25', () => {
    const now = new Date(2026, 6, 25, 9, 0, 0)

    expect(nextWorkdayReviewDelay(now)).toEqual({
      localDate: '2026-07-27',
      delayMs: 206_700_000,
    })
  })

  it('adds one local day without timezone drift', () => {
    expect(addLocalDays('2026-07-20', 1)).toBe('2026-07-21')
  })

  it('includes every task in group and task order, including 100 percent tasks', () => {
    const items = buildDailyTaskReviewItems(day())

    expect(items.map((item) => `${item.code}:${item.title}:${item.progress}`)).toEqual([
      'L36:Shot refinement:100',
      'L36:Export review:40',
      'A12:Sound pass:20',
    ])
  })

  it('checks next-day duplicates by exact title only', () => {
    expect(hasExactTitle(day(), 'Shot refinement')).toBe(true)
    expect(hasExactTitle(day(), 'shot refinement')).toBe(false)
    expect(hasExactTitle(day(), 'Shot refinement ')).toBe(false)
  })
})

function day(): DailyTaskDay {
  return {
    id: 'day-1',
    localDate: '2026-07-20',
    settledAt: null,
    reportSnapshot: null,
    groups: [
      {
        id: 'group-1',
        code: 'L36',
        projectId: null,
        position: 0,
        tasks: [
          task('task-1', 'Shot refinement', 100, 0),
          task('task-2', 'Export review', 40, 1),
        ],
      },
      {
        id: 'group-2',
        code: 'A12',
        projectId: 'project-1',
        position: 1,
        tasks: [
          task('task-3', 'Sound pass', 20, 0),
        ],
      },
    ],
  }
}

function task(id: string, title: string, progress: number, position: number) {
  return {
    id,
    title,
    progress,
    note: `Note for ${title}`,
    investedMinutes: 15,
    reminderTime: '18:00',
    reminderContent: `Reminder for ${title}`,
    position,
    sourceTaskId: null,
    sourceSnapshotHash: null,
    createdAt: '2026-07-20T08:00:00Z',
    updatedAt: '2026-07-20T08:00:00Z',
  }
}

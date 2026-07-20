import type { DailyTask, DailyTaskDay } from '@/domain/production'

export const DAILY_TASK_REVIEW_HOUR = 18
export const DAILY_TASK_REVIEW_MINUTE = 25

export interface DailyTaskReviewItem {
  taskId: string
  code: string
  projectId: string | null
  title: string
  progress: number
  note: string
  investedMinutes: number
  reminderTime: string
  reminderContent: string
}

export interface NextReviewSchedule {
  localDate: string
  delayMs: number
}

export function isWorkday(value: Date) {
  const day = value.getDay()
  return day >= 1 && day <= 5
}

export function localDateFromDate(value: Date) {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function addLocalDays(localDate: string, days: number) {
  const [year, month, day] = localDate.split('-').map(Number)
  const value = new Date(year, month - 1, day + days, 0, 0, 0, 0)
  return localDateFromDate(value)
}

export function nextWorkdayReviewDelay(now = new Date()): NextReviewSchedule {
  for (let offset = 0; offset <= 7; offset += 1) {
    const candidate = new Date(
      now.getFullYear(),
      now.getMonth(),
      now.getDate() + offset,
      DAILY_TASK_REVIEW_HOUR,
      DAILY_TASK_REVIEW_MINUTE,
      0,
      0,
    )
    if (!isWorkday(candidate)) continue
    if (candidate.getTime() <= now.getTime()) continue
    return {
      localDate: localDateFromDate(candidate),
      delayMs: candidate.getTime() - now.getTime(),
    }
  }

  const fallback = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate() + 1,
    DAILY_TASK_REVIEW_HOUR,
    DAILY_TASK_REVIEW_MINUTE,
    0,
    0,
  )
  return {
    localDate: localDateFromDate(fallback),
    delayMs: Math.max(0, fallback.getTime() - now.getTime()),
  }
}

export function buildDailyTaskReviewItems(day: DailyTaskDay): DailyTaskReviewItem[] {
  return [...day.groups]
    .sort((left, right) => left.position - right.position || left.id.localeCompare(right.id))
    .flatMap((group) =>
      [...group.tasks]
        .sort((left, right) => left.position - right.position || left.id.localeCompare(right.id))
        .map((task) => reviewItemFromTask(task, group.code, group.projectId)),
    )
}

export function hasExactTitle(day: DailyTaskDay, title: string) {
  return day.groups.some((group) => group.tasks.some((task) => task.title === title))
}

function reviewItemFromTask(
  task: DailyTask,
  code: string,
  projectId: string | null,
): DailyTaskReviewItem {
  return {
    taskId: task.id,
    code,
    projectId,
    title: task.title,
    progress: task.progress,
    note: task.note,
    investedMinutes: task.investedMinutes,
    reminderTime: task.reminderTime,
    reminderContent: task.reminderContent,
  }
}

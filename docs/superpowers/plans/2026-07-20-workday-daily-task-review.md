# Workday Daily Task Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a workday 18:25 floating-window review flow that asks the user to complete or delay every current-day task, then copies the daily report.

**Architecture:** Add a small pure helper module for date and task-review shaping, then wire a coordinator in `App.vue` that owns scheduling and data mutation. `FloatButton.vue` only renders the review card and emits user actions back to `App.vue`; it does not call the daily task database directly.

**Tech Stack:** Vue 3, Pinia, Tauri event API, existing `productionIpc` daily-task commands, Vitest, Vue Test Utils.

---

## File Structure

- Create `src/lib/dailyTaskReview.ts`
  - Pure helpers for workday checks, 18:25 scheduling, next local date, flattening `DailyTaskDay` into review items, and exact-title duplicate detection.
- Create `tests/lib/dailyTaskReview.test.ts`
  - Unit tests for the pure helper module.
- Modify `src/App.vue`
  - Add the workday review timer.
  - Load current-day tasks at 18:25 only while the app is running.
  - Emit review payloads to the floating window.
  - Handle complete, delay, copy-report, and dismiss events from the floating window.
- Modify `src/components/FloatButton.vue`
  - Render the daily task review card.
  - Emit review actions back to the main window.
  - Reuse the existing floating reminder sizing and transparent-background behavior.
- Modify `tests/components/App.test.ts`
  - Add coordinator tests for schedule, task mutation, delay duplicate prevention, and report copy.
- Modify `tests/components/FloatButton.test.ts`
  - Add UI event tests for the review card.

---

### Task 1: Pure Daily Review Helpers

**Files:**
- Create: `src/lib/dailyTaskReview.ts`
- Test: `tests/lib/dailyTaskReview.test.ts`

- [ ] **Step 1: Write the failing helper tests**

Create `tests/lib/dailyTaskReview.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import {
  addLocalDays,
  buildDailyTaskReviewItems,
  hasExactTitle,
  isWorkday,
  nextWorkdayReviewDelay,
} from '@/lib/dailyTaskReview'
import type { DailyTaskDay } from '@/domain/production'

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
```

- [ ] **Step 2: Run the helper tests to verify RED**

Run:

```powershell
pnpm vitest run tests/lib/dailyTaskReview.test.ts
```

Expected: fail because `src/lib/dailyTaskReview.ts` does not exist.

- [ ] **Step 3: Implement the helper module**

Create `src/lib/dailyTaskReview.ts`:

```ts
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
```

- [ ] **Step 4: Run helper tests to verify GREEN**

Run:

```powershell
pnpm vitest run tests/lib/dailyTaskReview.test.ts
```

Expected: pass.

- [ ] **Step 5: Commit helper module**

Run:

```powershell
git add src/lib/dailyTaskReview.ts tests/lib/dailyTaskReview.test.ts
git commit -m "feat: add daily task review helpers"
```

---

### Task 2: Main Window Review Coordinator

**Files:**
- Modify: `src/App.vue`
- Test: `tests/components/App.test.ts`

- [ ] **Step 1: Extend App tests with failing coordinator coverage**

In `tests/components/App.test.ts`, extend the existing mocks:

Add this import near the current store imports:

```ts
import * as ipc from '@/lib/ipc'
```

```ts
const productionIpc = vi.hoisted(() => ({
  loadDailyTaskDay: vi.fn(),
  createDailyTask: vi.fn(),
  updateDailyTask: vi.fn(),
  getDailyReport: vi.fn(),
}))

vi.mock('@/lib/productionIpc', () => productionIpc)
```

Also add `copyToClipboard` to the existing `@/lib/ipc` mock:

```ts
copyToClipboard: vi.fn().mockResolvedValue(undefined),
```

Add these tests inside `describe('App', () => { ... })`:

```ts
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
```

Add these helpers at the bottom of `tests/components/App.test.ts`:

```ts
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
  const day = reviewDay(localDate)
  day.groups[0].tasks = [reviewTask('task-1', 'Already done', 100, 'Completed before review', 0)]
  return day
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
```

- [ ] **Step 2: Run App tests to verify RED**

Run:

```powershell
pnpm vitest run tests/components/App.test.ts
```

Expected: fail because `App.vue` does not schedule or handle daily task review events yet.

- [ ] **Step 3: Implement coordinator state and event types in `App.vue`**

Add imports near the top of `src/App.vue`:

```ts
import { copyToClipboard } from '@/lib/ipc'
import {
  addLocalDays,
  buildDailyTaskReviewItems,
  nextWorkdayReviewDelay,
  hasExactTitle,
  type DailyTaskReviewItem,
} from '@/lib/dailyTaskReview'
import {
  createDailyTask as createDailyTaskDirect,
  getDailyReport,
  loadDailyTaskDay,
  updateDailyTask as updateDailyTaskDirect,
} from '@/lib/productionIpc'
```

Add module state near the existing reminder timer state:

```ts
let dailyTaskReviewTimer: ReturnType<typeof window.setTimeout> | null = null
let activeDailyTaskReview: {
  sessionId: string
  localDate: string
  tasks: DailyTaskReviewItem[]
  index: number
} | null = null
const firedDailyTaskReviewDates = new Set<string>()
```

Add payload guards:

```ts
interface DailyTaskReviewActionPayload {
  sessionId: string
  taskId?: string
}

function isDailyTaskReviewActionPayload(value: unknown): value is DailyTaskReviewActionPayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<DailyTaskReviewActionPayload>
  return typeof payload.sessionId === 'string' &&
    (payload.taskId === undefined || typeof payload.taskId === 'string')
}
```

- [ ] **Step 4: Add scheduling and session emit helpers in `App.vue`**

Add these functions near the existing daily reminder functions:

```ts
function clearDailyTaskReviewTimer() {
  if (dailyTaskReviewTimer !== null) {
    window.clearTimeout(dailyTaskReviewTimer)
    dailyTaskReviewTimer = null
  }
}

function scheduleDailyTaskReview() {
  clearDailyTaskReviewTimer()
  const next = nextWorkdayReviewDelay(new Date())
  const delay = Math.max(0, Math.min(next.delayMs, 2_147_483_647))
  dailyTaskReviewTimer = window.setTimeout(() => {
    void startDailyTaskReview(next.localDate)
  }, delay)
}

async function startDailyTaskReview(localDate: string) {
  scheduleDailyTaskReview()
  if (firedDailyTaskReviewDates.has(localDate)) return
  firedDailyTaskReviewDates.add(localDate)
  try {
    const day = await loadDailyTaskDay(localDate)
    const tasks = buildDailyTaskReviewItems(day)
    if (tasks.length === 0) return
    activeDailyTaskReview = {
      sessionId: crypto.randomUUID(),
      localDate,
      tasks,
      index: 0,
    }
    await emitDailyTaskReview('daily-task-review-start')
  } catch {
    ui.showToast('每日任务确认启动失败')
  }
}

async function emitDailyTaskReview(eventName = 'daily-task-review-update') {
  if (!activeDailyTaskReview) return
  const task = activeDailyTaskReview.tasks[activeDailyTaskReview.index]
  if (!task) {
    await emitTo('floatbtn', 'daily-task-review-complete', {
      sessionId: activeDailyTaskReview.sessionId,
      localDate: activeDailyTaskReview.localDate,
    })
    return
  }
  await emitTo('floatbtn', eventName, {
    sessionId: activeDailyTaskReview.sessionId,
    localDate: activeDailyTaskReview.localDate,
    index: activeDailyTaskReview.index,
    total: activeDailyTaskReview.tasks.length,
    task,
  })
}

function currentReviewTask(payload: DailyTaskReviewActionPayload) {
  if (!activeDailyTaskReview) return null
  if (payload.sessionId !== activeDailyTaskReview.sessionId) return null
  const task = activeDailyTaskReview.tasks[activeDailyTaskReview.index]
  if (!task) return null
  if (payload.taskId && payload.taskId !== task.taskId) return null
  return task
}

async function advanceDailyTaskReview() {
  if (!activeDailyTaskReview) return
  activeDailyTaskReview.index += 1
  await emitDailyTaskReview()
}
```

- [ ] **Step 5: Add complete, delay, report, and dismiss handlers in `App.vue`**

Add these functions:

```ts
async function completeDailyTaskReviewTask(payload: DailyTaskReviewActionPayload) {
  const task = currentReviewTask(payload)
  if (!task || !activeDailyTaskReview) return
  try {
    const updatedDay = await updateDailyTaskDirect({
      taskId: task.taskId,
      title: task.title,
      progress: 100,
      note: task.note,
      investedMinutes: task.investedMinutes,
      reminderTime: task.reminderTime,
      reminderContent: task.reminderContent,
    })
    if (daily.selectedDate === activeDailyTaskReview.localDate) daily.day = updatedDay
    await advanceDailyTaskReview()
  } catch {
    await emitTo('floatbtn', 'daily-task-review-error', {
      sessionId: activeDailyTaskReview.sessionId,
      message: '完成任务失败，请重试',
    })
  }
}

async function delayDailyTaskReviewTask(payload: DailyTaskReviewActionPayload) {
  const task = currentReviewTask(payload)
  if (!task || !activeDailyTaskReview) return
  const nextDate = addLocalDays(activeDailyTaskReview.localDate, 1)
  try {
    const nextDay = await loadDailyTaskDay(nextDate)
    if (!hasExactTitle(nextDay, task.title)) {
      const updatedNextDay = await createDailyTaskDirect({
        localDate: nextDate,
        code: task.code,
        projectId: task.projectId,
        title: task.title,
        progress: task.progress,
        note: task.note,
        investedMinutes: task.investedMinutes,
        reminderTime: task.reminderTime,
        reminderContent: task.reminderContent,
      })
      if (daily.selectedDate === nextDate) daily.day = updatedNextDay
    }
    await advanceDailyTaskReview()
  } catch {
    await emitTo('floatbtn', 'daily-task-review-error', {
      sessionId: activeDailyTaskReview.sessionId,
      message: '延后任务失败，请重试',
    })
  }
}

async function copyDailyTaskReviewReport(payload: DailyTaskReviewActionPayload) {
  if (!activeDailyTaskReview || payload.sessionId !== activeDailyTaskReview.sessionId) return
  try {
    const report = await getDailyReport(activeDailyTaskReview.localDate)
    await copyToClipboard(report.text)
    await emitTo('floatbtn', 'daily-task-review-close', {
      sessionId: activeDailyTaskReview.sessionId,
    })
    activeDailyTaskReview = null
  } catch {
    await emitTo('floatbtn', 'daily-task-review-error', {
      sessionId: activeDailyTaskReview.sessionId,
      message: '复制日报失败，请重试',
    })
  }
}

function dismissDailyTaskReview(payload: DailyTaskReviewActionPayload) {
  if (!activeDailyTaskReview || payload.sessionId !== activeDailyTaskReview.sessionId) return
  activeDailyTaskReview = null
}
```

- [ ] **Step 6: Register and clean up review listeners in `App.vue`**

Add listener variables near the existing unlisten variables:

```ts
let unlistenReviewCompleteTask: UnlistenFn | null = null
let unlistenReviewDelayTask: UnlistenFn | null = null
let unlistenReviewCopyReport: UnlistenFn | null = null
let unlistenReviewDismiss: UnlistenFn | null = null
```

Inside `onMounted`, after existing reminder listeners:

```ts
unlistenReviewCompleteTask = await listen('daily-task-review-complete-task', (event) => {
  if (!isDailyTaskReviewActionPayload(event.payload)) return
  void completeDailyTaskReviewTask(event.payload)
})
unlistenReviewDelayTask = await listen('daily-task-review-delay-task', (event) => {
  if (!isDailyTaskReviewActionPayload(event.payload)) return
  void delayDailyTaskReviewTask(event.payload)
})
unlistenReviewCopyReport = await listen('daily-task-review-copy-report', (event) => {
  if (!isDailyTaskReviewActionPayload(event.payload)) return
  void copyDailyTaskReviewReport(event.payload)
})
unlistenReviewDismiss = await listen('daily-task-review-dismiss', (event) => {
  if (!isDailyTaskReviewActionPayload(event.payload)) return
  dismissDailyTaskReview(event.payload)
})
scheduleDailyTaskReview()
```

Inside `onUnmounted`:

```ts
clearDailyTaskReviewTimer()
unlistenReviewCompleteTask?.()
unlistenReviewCompleteTask = null
unlistenReviewDelayTask?.()
unlistenReviewDelayTask = null
unlistenReviewCopyReport?.()
unlistenReviewCopyReport = null
unlistenReviewDismiss?.()
unlistenReviewDismiss = null
activeDailyTaskReview = null
```

- [ ] **Step 7: Run App tests to verify GREEN**

Run:

```powershell
pnpm vitest run tests/components/App.test.ts
```

Expected: pass.

- [ ] **Step 8: Commit coordinator**

Run:

```powershell
git add src/App.vue tests/components/App.test.ts
git commit -m "feat: coordinate workday daily task review"
```

---

### Task 3: Floating Window Review UI

**Files:**
- Modify: `src/components/FloatButton.vue`
- Test: `tests/components/FloatButton.test.ts`

- [ ] **Step 1: Add failing FloatButton tests**

Add tests to `tests/components/FloatButton.test.ts`:

```ts
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
```

Add this helper in `tests/components/FloatButton.test.ts`:

```ts
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
```

- [ ] **Step 2: Run FloatButton tests to verify RED**

Run:

```powershell
pnpm vitest run tests/components/FloatButton.test.ts
```

Expected: fail because `FloatButton.vue` does not render daily task review events yet.

- [ ] **Step 3: Add review state and payload guards in `FloatButton.vue`**

Add these interfaces and refs after `DailyTaskReminderPayload`:

```ts
interface FloatingDailyTaskReviewItem {
  taskId: string
  code: string
  title: string
  progress: number
  note: string
}

interface FloatingDailyTaskReviewPayload {
  sessionId: string
  localDate: string
  index: number
  total: number
  task: FloatingDailyTaskReviewItem
}

interface FloatingDailyTaskReviewCompletePayload {
  sessionId: string
  localDate: string
}

const activeDailyReview = ref<FloatingDailyTaskReviewPayload | null>(null)
const reviewComplete = ref<FloatingDailyTaskReviewCompletePayload | null>(null)
const reviewError = ref('')
let unlistenDailyReviewStart: UnlistenFn | null = null
let unlistenDailyReviewUpdate: UnlistenFn | null = null
let unlistenDailyReviewComplete: UnlistenFn | null = null
let unlistenDailyReviewError: UnlistenFn | null = null
let unlistenDailyReviewClose: UnlistenFn | null = null
```

Add guards:

```ts
function isFloatingDailyTaskReviewPayload(value: unknown): value is FloatingDailyTaskReviewPayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<FloatingDailyTaskReviewPayload>
  const task = payload.task as Partial<FloatingDailyTaskReviewItem> | undefined
  return typeof payload.sessionId === 'string' &&
    typeof payload.localDate === 'string' &&
    typeof payload.index === 'number' &&
    typeof payload.total === 'number' &&
    !!task &&
    typeof task.taskId === 'string' &&
    typeof task.code === 'string' &&
    typeof task.title === 'string' &&
    typeof task.progress === 'number' &&
    typeof task.note === 'string'
}

function isFloatingDailyTaskReviewCompletePayload(
  value: unknown,
): value is FloatingDailyTaskReviewCompletePayload {
  if (!value || typeof value !== 'object') return false
  const payload = value as Partial<FloatingDailyTaskReviewCompletePayload>
  return typeof payload.sessionId === 'string' && typeof payload.localDate === 'string'
}
```

- [ ] **Step 4: Add review window behavior in `FloatButton.vue`**

Add:

```ts
async function expandForDailyReview() {
  const position = await win.outerPosition()
  compactPosition = { x: position.x, y: position.y }
  await win.setSize(new LogicalSize(380, 220))
  await win.setPosition(new PhysicalPosition(position.x - 316, position.y))
  reminderExpanded.value = true
}

async function showDailyReview(payload: FloatingDailyTaskReviewPayload) {
  activeReminder.value = null
  activeDailyReview.value = payload
  reviewComplete.value = null
  reviewError.value = ''
  try {
    await expandForDailyReview()
  } catch {
    // Browser tests and some native states can reject resize; the card still renders.
  }
}

async function showDailyReviewComplete(payload: FloatingDailyTaskReviewCompletePayload) {
  activeReminder.value = null
  activeDailyReview.value = null
  reviewComplete.value = payload
  reviewError.value = ''
  try {
    await expandForDailyReview()
  } catch {
    // Browser tests and some native states can reject resize; the card still renders.
  }
}

async function closeDailyReview() {
  const sessionId = activeDailyReview.value?.sessionId ?? reviewComplete.value?.sessionId
  activeDailyReview.value = null
  reviewComplete.value = null
  reviewError.value = ''
  if (sessionId) {
    await emitTo('main', 'daily-task-review-dismiss', { sessionId })
  }
  try {
    await restoreReminderSize()
  } catch {
    // Keep click handling responsive when native resize is unavailable.
  }
}

async function clearDailyReviewFromMain(sessionId: string) {
  if (activeDailyReview.value?.sessionId !== sessionId && reviewComplete.value?.sessionId !== sessionId) {
    return
  }
  activeDailyReview.value = null
  reviewComplete.value = null
  reviewError.value = ''
  try {
    await restoreReminderSize()
  } catch {
    // Keep click handling responsive when native resize is unavailable.
  }
}

async function completeReviewTask() {
  if (!activeDailyReview.value) return
  await emitTo('main', 'daily-task-review-complete-task', {
    sessionId: activeDailyReview.value.sessionId,
    taskId: activeDailyReview.value.task.taskId,
  })
}

async function delayReviewTask() {
  if (!activeDailyReview.value) return
  await emitTo('main', 'daily-task-review-delay-task', {
    sessionId: activeDailyReview.value.sessionId,
    taskId: activeDailyReview.value.task.taskId,
  })
}

async function copyDailyReviewReport() {
  if (!reviewComplete.value) return
  await emitTo('main', 'daily-task-review-copy-report', {
    sessionId: reviewComplete.value.sessionId,
  })
}
```

- [ ] **Step 5: Register review listeners in `FloatButton.vue`**

Add to the `Promise.all` in `onMounted`:

```ts
listen('daily-task-review-start', (event) => {
  if (!isFloatingDailyTaskReviewPayload(event.payload)) return
  void showDailyReview(event.payload)
}),
listen('daily-task-review-update', (event) => {
  if (!isFloatingDailyTaskReviewPayload(event.payload)) return
  void showDailyReview(event.payload)
}),
listen('daily-task-review-complete', (event) => {
  if (!isFloatingDailyTaskReviewCompletePayload(event.payload)) return
  void showDailyReviewComplete(event.payload)
}),
listen('daily-task-review-error', (event) => {
  const payload = event.payload as { message?: unknown }
  reviewError.value = typeof payload.message === 'string' ? payload.message : '操作失败，请重试'
}),
listen('daily-task-review-close', (event) => {
  const payload = event.payload as { sessionId?: unknown }
  if (typeof payload.sessionId !== 'string') return
  void clearDailyReviewFromMain(payload.sessionId)
}),
```

Assign the returned unlisten functions to the variables from Step 3. Clean them in `onUnmounted`:

```ts
unlistenDailyReviewStart?.()
unlistenDailyReviewUpdate?.()
unlistenDailyReviewComplete?.()
unlistenDailyReviewError?.()
unlistenDailyReviewClose?.()
unlistenDailyReviewStart = null
unlistenDailyReviewUpdate = null
unlistenDailyReviewComplete = null
unlistenDailyReviewError = null
unlistenDailyReviewClose = null
```

- [ ] **Step 6: Add review card markup in `FloatButton.vue`**

Add this section after the existing reminder `<section>`:

```vue
<section
  v-if="activeDailyReview || reviewComplete"
  class="floating-reminder floating-daily-review"
  data-floating-daily-review
  role="alertdialog"
  aria-live="assertive"
  @mousedown.stop
  @click.stop
>
  <template v-if="activeDailyReview">
    <header>
      <p>{{ activeDailyReview.localDate }} · {{ activeDailyReview.index + 1 }} / {{ activeDailyReview.total }}</p>
      <h2>每日任务确认</h2>
    </header>
    <div class="daily-review-task">
      <strong>{{ activeDailyReview.task.title }}</strong>
      <span>#{{ activeDailyReview.task.code }} · {{ activeDailyReview.task.progress }}%</span>
      <p v-if="activeDailyReview.task.note.trim()">
        {{ activeDailyReview.task.note }}
      </p>
    </div>
    <p
      v-if="reviewError"
      class="floating-review-error"
      role="alert"
    >
      {{ reviewError }}
    </p>
    <footer>
      <button
        class="primary"
        type="button"
        data-action="daily-review-complete"
        @click="completeReviewTask"
      >
        完成
      </button>
      <button
        type="button"
        data-action="daily-review-delay"
        @click="delayReviewTask"
      >
        延后
      </button>
      <button
        type="button"
        data-action="daily-review-close"
        @click="closeDailyReview"
      >
        关闭
      </button>
    </footer>
  </template>
  <template v-else-if="reviewComplete">
    <header>
      <p>{{ reviewComplete.localDate }}</p>
      <h2>全部确认完成</h2>
    </header>
    <p>可以复制今天的日报。</p>
    <p
      v-if="reviewError"
      class="floating-review-error"
      role="alert"
    >
      {{ reviewError }}
    </p>
    <footer>
      <button
        class="primary"
        type="button"
        data-action="daily-review-copy-report"
        @click="copyDailyReviewReport"
      >
        复制日报
      </button>
      <button
        type="button"
        data-action="daily-review-close"
        @click="closeDailyReview"
      >
        关闭
      </button>
    </footer>
  </template>
</section>
```

- [ ] **Step 7: Add compact review styles in `FloatButton.vue`**

Add:

```css
.floating-daily-review {
  width: 296px;
  max-height: 204px;
}

.daily-review-task {
  display: grid;
  gap: 4px;
}

.daily-review-task strong {
  color: var(--bb-text);
  font-size: 13px;
  line-height: 1.35;
  overflow-wrap: anywhere;
}

.daily-review-task span {
  color: var(--bb-primary);
  font: 10px var(--bb-mono);
}

.daily-review-task p,
.floating-review-error {
  margin: 0;
  color: var(--bb-text-soft);
  font-size: 12px;
  line-height: 1.4;
  overflow-wrap: anywhere;
}

.floating-review-error {
  color: #ffb6c0;
}

.floating-daily-review footer {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}
```

- [ ] **Step 8: Run FloatButton tests to verify GREEN**

Run:

```powershell
pnpm vitest run tests/components/FloatButton.test.ts
```

Expected: pass.

- [ ] **Step 9: Commit floating review UI**

Run:

```powershell
git add src/components/FloatButton.vue tests/components/FloatButton.test.ts
git commit -m "feat: add floating daily task review UI"
```

---

### Task 4: Integration Verification

**Files:**
- Modify only if tests expose an issue:
  - `src/App.vue`
  - `src/components/FloatButton.vue`
  - `src/lib/dailyTaskReview.ts`

- [ ] **Step 1: Run focused review tests**

Run:

```powershell
pnpm vitest run tests/lib/dailyTaskReview.test.ts tests/components/App.test.ts tests/components/FloatButton.test.ts tests/components/DailyTasksPage.test.ts
```

Expected: all selected test files pass.

- [ ] **Step 2: Run full frontend verification**

Run:

```powershell
pnpm check
```

Expected: typecheck, lint, and all Vitest tests pass.

- [ ] **Step 3: Build the frontend**

Run:

```powershell
pnpm build
```

Expected: Vue typecheck and Vite production build pass.

- [ ] **Step 4: Manual debug check**

Run the app in debug mode:

```powershell
pnpm tauri dev
```

Manual check:

1. Temporarily use browser/Vitest fake time or temporarily adjust the review helper time only in a local throwaway edit.
2. Confirm the floating window shows the task review card.
3. Confirm `完成` updates progress to `100%`.
4. Confirm `延后` creates tomorrow's task and does not duplicate exact-title tasks.
5. Confirm the final `复制日报` button copies the report.
6. Revert any temporary local throwaway edit before committing.

- [ ] **Step 5: Commit final fixes if needed**

If Task 4 required fixes, run:

```powershell
git add src/App.vue src/components/FloatButton.vue src/lib/dailyTaskReview.ts tests/lib/dailyTaskReview.test.ts tests/components/App.test.ts tests/components/FloatButton.test.ts
git commit -m "fix: stabilize workday daily task review"
```

If no fixes were needed, do not create an empty commit.

---

## Self-Review

Spec coverage:

- Workday 18:25 trigger: Task 1 and Task 2.
- No catch-up when app was closed or started late: Task 1 and Task 2.
- Include every current-day task, including `100%`: Task 1 and Task 2.
- Complete action sets current progress to `100%`: Task 2.
- Delay action creates tomorrow's task with same progress and metadata: Task 2.
- Exact-title duplicate prevention: Task 1 and Task 2.
- Floating window sequential UI: Task 3.
- Final copy report action: Task 2 and Task 3.
- No schema change: File structure and architecture use existing IPC commands.

Placeholder scan:

- The plan contains no open placeholders or unspecified implementation steps.

Type consistency:

- The plan uses `DailyTaskReviewItem` for coordinator data.
- Floating-window payloads intentionally use a narrower `FloatingDailyTaskReviewItem`.
- Event names match across App and FloatButton tasks.

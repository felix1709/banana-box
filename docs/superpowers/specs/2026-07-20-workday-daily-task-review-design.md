# Workday Daily Task Review Design

## Goal

Add a workday 18:25 daily task review flow that appears in the floating window, asks the user to confirm every task for the current day, and copies the daily report after all tasks are processed.

## User Workflow

On Monday through Friday at 18:25, if Banana Box is already running, the floating window opens a daily task review card.

The review card shows one task at a time. For each task, the user sees:

- Task code, such as `L36`
- Task title
- Current progress
- Current note, if any

The user chooses one action:

- `完成`: update the current day's task progress to `100%`.
- `延后`: create the same task for the next day, keeping the current progress, note, reminder time, and reminder content. If the next day already has a task with the exact same title, do not create a duplicate.

After the final task is processed, the floating window shows a `复制日报` button. Clicking it copies the selected day's daily report to the clipboard and closes the review flow.

## Trigger Rules

- Trigger only on workdays: Monday, Tuesday, Wednesday, Thursday, and Friday.
- Trigger at local time `18:25`.
- If the app is not running at 18:25, do not show a delayed or catch-up review later.
- If there are no tasks for the current day, do not open the review flow.
- The automatic review should trigger at most once per local date.
- If the user closes or dismisses the floating review flow, the flow ends for that day and should not automatically reopen.

## Task Selection Rules

The review includes every current-day task, including tasks that are already at `100%`.

Task order follows the daily task page:

1. Daily task groups by group position.
2. Tasks inside each group by task position.

This keeps the review order consistent with what the user sees in the daily task page.

## Completion Behavior

When the user clicks `完成`:

- The app updates the current task.
- The task title, note, invested minutes, reminder time, and reminder content stay unchanged.
- The task progress becomes `100%`.
- The flow moves to the next task.

If the update fails, the floating window keeps the current task visible and shows an error state. The user can retry the same action.

## Delay Behavior

When the user clicks `延后`:

- The app checks the next local date.
- If the next day already contains a task whose title exactly matches the current task title, the app does not create a duplicate.
- If no exact title match exists, the app creates a new task for the next day.
- The new task keeps:
  - Same group code
  - Same title
  - Same progress
  - Same note
  - Same invested minutes
  - Same reminder time
  - Same reminder content
- The current day's task is not modified by the delay action.
- The flow moves to the next task.

The duplicate check is title-only and exact-match. It is case-sensitive and does not trim or normalize beyond the title already stored in the daily task.

If creating the next-day task fails, the floating window keeps the current task visible and shows an error state. The user can retry the same action.

## Daily Report Behavior

After all tasks are reviewed:

- The floating window shows a final state with a `复制日报` button.
- Clicking the button calls the existing daily report generator for the original review date.
- The generated report text is copied to the clipboard.
- If copying succeeds, the review flow closes.
- If copying fails, the final state stays visible and shows an error state so the user can retry.

## UI Design

Use the existing floating reminder window as the base pattern.

The floating review card should remain compact and operational:

- Header: `每日任务确认`
- Subheader: current date and `当前第 N / 总数 M 项`
- Main title: task title
- Metadata row: task code and progress
- Optional note line
- Footer actions:
  - `完成`
  - `延后`
  - `关闭`

The final state replaces the task actions with:

- `复制日报`
- `关闭`

The card must fit inside the floating window and scroll internally if the task title or note is long.

## Architecture

The feature should reuse the existing daily task store, Tauri IPC functions, floating-window event channel, and report-copying utilities.

Add a small front-end review coordinator in `App.vue` or a focused helper module. Its job is to:

- Schedule the next workday 18:25 timer while the app is running.
- Collect the current day's tasks at trigger time.
- Emit a new floating-window event with the review session payload.
- Listen for floating-window actions and update daily task data.
- Emit progress updates back to the floating window.

Update `FloatButton.vue` to render the review session. It should not directly mutate tasks. It should only send user decisions back to the main window through events.

Extend `dailyTasks.ts` with helper actions only if needed. Prefer using existing `selectDate`, `update`, and `create` actions unless the implementation becomes hard to read.

## Events

Use separate events from the existing single-task reminder flow so the two behaviors stay easy to reason about.

Suggested event names:

- Main to floating window:
  - `daily-task-review-start`
  - `daily-task-review-update`
  - `daily-task-review-complete`
  - `daily-task-review-error`
- Floating window to main:
  - `daily-task-review-complete-task`
  - `daily-task-review-delay-task`
  - `daily-task-review-copy-report`
  - `daily-task-review-dismiss`

The event payload should include a stable `sessionId` so stale clicks from an older floating-window session cannot modify the wrong review.

## Data Boundaries

No database schema change is needed.

The next-day delayed task can be created with the existing `create_daily_task` command. The exact-title duplicate check can be done in the frontend by loading the next day's `DailyTaskDay` before creating the task.

## Testing

Add tests for:

- Workday 18:25 schedules and emits a review session only when the app is running at that time.
- Weekend dates do not trigger the review.
- A missed 18:25 does not trigger a catch-up review after app startup.
- The review includes all tasks, including `100%` tasks.
- Clicking `完成` updates the task progress to `100%`.
- Clicking `延后` creates a next-day task with the same progress and metadata.
- Clicking `延后` does not create a duplicate when the next day already has the exact same title.
- After the last task, the floating window shows the report-copy action.
- Clicking `复制日报` copies the report text and closes the review.

## Out Of Scope

- No settings page for changing the 18:25 time.
- No weekend reminders.
- No catch-up reminders when the app was closed at 18:25.
- No cloud notification for this local reminder flow.
- No automatic daily settlement or locking.

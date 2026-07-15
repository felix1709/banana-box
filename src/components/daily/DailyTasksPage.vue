<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { AlarmClock, Check, ChevronLeft, ChevronRight, Copy, Plus, Trash2, X } from '@lucide/vue'
import type { DailyTask } from '@/domain/production'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useUiStore } from '@/stores/ui'
import { getDailyReport } from '@/lib/productionIpc'
import { copyToClipboard } from '@/lib/ipc'

const daily = useDailyTasksStore()
const ui = useUiStore()
const draft = reactive({ code: '', title: '', progress: 0, note: '' })
const copyingReport = ref(false)
const reportCopied = ref(false)
const reminderTaskId = ref<string | null>(null)
const reminderTrigger = ref<HTMLButtonElement | null>(null)
const reminderPopover = ref<HTMLElement | null>(null)
const reminderPopoverStyle = ref<Record<string, string>>({})
const reminderDraft = reactive({ time: '', content: '' })
const createOpen = ref(false)
const createTrigger = ref<HTMLButtonElement | null>(null)
const createPopover = ref<HTMLElement | null>(null)
const createPopoverStyle = ref<Record<string, string>>({})
const editingTaskId = ref<string | null>(null)
const editDraft = reactive({ title: '', note: '', reminderTime: '', reminderContent: '' })
let reportCopyResetTimer: ReturnType<typeof window.setTimeout> | null = null

const reminderTask = computed(() => findTask(reminderTaskId.value))
const editingTask = computed(() => findTask(editingTaskId.value))

function moveDate(days: number) {
  const [year, month, day] = daily.selectedDate.split('-').map(Number)
  const value = new Date(Date.UTC(year, month - 1, day + days))
  return daily.selectDate(value.toISOString().slice(0, 10))
}

async function createTask() {
  if (!draft.code.trim() || !draft.title.trim()) return
  await daily.create({ ...draft, code: draft.code.trim(), title: draft.title.trim(), investedMinutes: 0 })
  Object.assign(draft, { code: '', title: '', progress: 0, note: '' })
  closeCreatePopover()
}

function findTask(taskId: string | null) {
  if (!taskId) return null
  for (const group of daily.day?.groups ?? []) {
    const task = group.tasks.find((item) => item.id === taskId)
    if (task) return task
  }
  return null
}

function updateInput(task: DailyTask, patch: Partial<DailyTask> = {}) {
  return {
    taskId: task.id,
    title: patch.title ?? task.title,
    progress: patch.progress ?? task.progress,
    note: patch.note ?? task.note,
    investedMinutes: patch.investedMinutes ?? task.investedMinutes,
    reminderTime: patch.reminderTime ?? task.reminderTime ?? '',
    reminderContent: patch.reminderContent ?? task.reminderContent ?? '',
  }
}

async function persistTask(task: DailyTask, patch: Partial<DailyTask> = {}) {
  if (daily.day?.settledAt) return
  await daily.update(updateInput(task, patch))
}

async function copyDailyReport() {
  if (copyingReport.value) return
  copyingReport.value = true
  resetReportCopied()
  try {
    const report = await getDailyReport(daily.selectedDate)
    await copyToClipboard(report.text)
    showReportCopied()
  } catch {
    resetReportCopied()
    ui.showToast('复制日报失败')
  } finally {
    copyingReport.value = false
  }
}

function resetReportCopied() {
  if (reportCopyResetTimer !== null) window.clearTimeout(reportCopyResetTimer)
  reportCopyResetTimer = null
  reportCopied.value = false
}

function showReportCopied() {
  resetReportCopied()
  reportCopied.value = true
  reportCopyResetTimer = window.setTimeout(resetReportCopied, 1200)
}

async function positionReminderPopover() {
  await nextTick()
  const rect = reminderTrigger.value?.getBoundingClientRect()
  if (!rect) return
  const width = Math.min(300, window.innerWidth - 24)
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, rect.right - width))
  const popoverHeight = reminderPopover.value?.offsetHeight || 220
  const bottomTop = rect.bottom + 8
  const top = bottomTop + popoverHeight <= window.innerHeight - 12
    ? bottomTop
    : Math.max(12, rect.top - popoverHeight - 8)
  reminderPopoverStyle.value = {
    position: 'fixed',
    zIndex: '240',
    top: `${top}px`,
    left: `${left}px`,
    width: `${width}px`,
  }
}

async function positionCreatePopover() {
  await nextTick()
  const rect = createTrigger.value?.getBoundingClientRect()
  if (!rect) return
  const width = Math.min(360, window.innerWidth - 24)
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, rect.right - width))
  createPopoverStyle.value = {
    position: 'fixed',
    zIndex: '240',
    top: `${rect.bottom + 8}px`,
    left: `${left}px`,
    width: `${width}px`,
  }
}

async function toggleCreatePopover(event: MouseEvent) {
  createTrigger.value = event.currentTarget as HTMLButtonElement
  createOpen.value = !createOpen.value
  if (createOpen.value) await positionCreatePopover()
}

function closeCreatePopover() {
  createOpen.value = false
  createTrigger.value = null
}

async function openReminder(task: DailyTask, event: MouseEvent) {
  reminderTaskId.value = task.id
  reminderTrigger.value = event.currentTarget as HTMLButtonElement
  reminderDraft.time = task.reminderTime ?? ''
  reminderDraft.content = task.reminderContent || task.title
  await positionReminderPopover()
}

function closeReminder() {
  reminderTaskId.value = null
  reminderTrigger.value = null
}

async function saveReminder() {
  const task = reminderTask.value
  if (!task) return
  await persistTask(task, {
    reminderTime: reminderDraft.time,
    reminderContent: reminderDraft.content.trim(),
  })
  closeReminder()
}

function closeReminderWhenClickingOutside(event: MouseEvent) {
  if (!reminderTaskId.value) return
  const target = event.target
  if (!(target instanceof Node)) return
  if (reminderTrigger.value?.contains(target)) return
  if (reminderPopover.value?.contains(target)) return
  closeReminder()
}

function closeCreateWhenClickingOutside(event: MouseEvent) {
  if (!createOpen.value) return
  const target = event.target
  if (!(target instanceof Node)) return
  if (createTrigger.value?.contains(target)) return
  if (createPopover.value?.contains(target)) return
  closeCreatePopover()
}

function openTaskEditor(task: DailyTask) {
  if (daily.day?.settledAt) return
  editingTaskId.value = task.id
  editDraft.title = task.title
  editDraft.note = task.note
  editDraft.reminderTime = task.reminderTime ?? ''
  editDraft.reminderContent = task.reminderContent ?? ''
}

function closeTaskEditor() {
  editingTaskId.value = null
}

async function saveTaskEditor() {
  const task = editingTask.value
  if (!task || !editDraft.title.trim()) return
  await persistTask(task, {
    title: editDraft.title.trim(),
    note: editDraft.note,
    reminderTime: editDraft.reminderTime,
    reminderContent: editDraft.reminderContent.trim(),
  })
  closeTaskEditor()
}

watch(() => daily.selectedDate, resetReportCopied)
onMounted(() => {
  void daily.selectDate(daily.selectedDate)
  document.addEventListener('click', closeReminderWhenClickingOutside)
  document.addEventListener('click', closeCreateWhenClickingOutside)
})
onBeforeUnmount(() => {
  resetReportCopied()
  document.removeEventListener('click', closeReminderWhenClickingOutside)
  document.removeEventListener('click', closeCreateWhenClickingOutside)
})
</script>

<template>
  <main class="daily-page">
    <header class="daily-toolbar">
      <div>
        <p>Daily ledger</p>
        <div class="daily-title-row">
          <h2>当日任务</h2>
          <button
            v-if="!daily.day?.settledAt"
            ref="createTrigger"
            class="daily-title-action"
            data-action="open-create-daily-task"
            type="button"
            title="新增当日任务"
            aria-label="新增当日任务"
            :aria-expanded="createOpen"
            @click="toggleCreatePopover"
          >
            <Plus :size="15" />
          </button>
        </div>
      </div>
      <div class="date-nav">
        <button
          data-action="previous-day"
          type="button"
          title="前一天"
          aria-label="前一天"
          @click="moveDate(-1)"
        >
          <ChevronLeft :size="16" />
        </button>
        <input
          v-model="daily.selectedDate"
          data-field="daily-date"
          type="date"
          @change="daily.selectDate(daily.selectedDate)"
        >
        <button
          data-action="next-day"
          type="button"
          title="后一天"
          aria-label="后一天"
          @click="moveDate(1)"
        >
          <ChevronRight :size="16" />
        </button>
        <button
          data-action="copy-daily-report"
          :data-copy-state="reportCopied ? 'copied' : 'ready'"
          type="button"
          :disabled="copyingReport"
          :title="reportCopied ? '已复制日报' : '复制日报'"
          :aria-label="reportCopied ? '已复制日报' : '复制日报'"
          @click="copyDailyReport"
        >
          <Check
            v-if="reportCopied"
            :size="16"
          />
          <Copy
            v-else
            :size="16"
          />
        </button>
      </div>
    </header>

    <div
      v-if="daily.error"
      class="daily-error"
      role="alert"
    >
      {{ daily.error }}
    </div>
    <div
      v-if="daily.day?.settledAt"
      class="daily-settled"
      data-settled-notice
    >
      本日已结算，重新打开结算后才能编辑。
    </div>

    <section class="daily-groups">
      <article
        v-for="group in daily.day?.groups ?? []"
        :key="group.id"
        class="daily-group"
        :data-task-group="group.code"
      >
        <header><strong>#{{ group.code }}</strong><span>{{ group.tasks.length }} 项</span></header>
        <div
          v-for="task in group.tasks"
          :key="task.id"
          class="daily-task task-card"
          :data-task-id="task.id"
          @dblclick="openTaskEditor(task)"
        >
          <div class="task-card-main">
            <span class="task-code">#{{ group.code }}</span>
            <input
              v-model="task.title"
              class="task-title"
              data-field="task-title"
              :disabled="Boolean(daily.day?.settledAt)"
              @change="persistTask(task)"
              @dblclick.stop
            >
            <label class="progress-control task-progress-control">
              <input
                v-model.number="task.progress"
                class="task-progress-range"
                data-field="task-progress"
                min="0"
                max="100"
                type="range"
                :style="{ '--task-progress': `${task.progress}%` }"
                :disabled="Boolean(daily.day?.settledAt)"
                @change="persistTask(task)"
                @dblclick.stop
              >
              <output data-progress-value>{{ task.progress }}%</output>
            </label>
            <div
              v-if="!daily.day?.settledAt"
              class="task-card-actions"
            >
              <button
                class="task-reminder-button"
                data-action="task-reminder"
                type="button"
                :class="{ active: task.reminderTime }"
                title="设置提醒"
                aria-label="设置提醒"
                @click.stop="openReminder(task, $event)"
              >
                <AlarmClock :size="15" />
              </button>
              <button
                data-action="delete-task"
                type="button"
                title="删除任务"
                aria-label="删除任务"
                @click.stop="daily.remove(task.id)"
              >
                <Trash2 :size="15" />
              </button>
            </div>
          </div>
          <p
            v-if="task.note.trim()"
            class="task-note"
            data-field="task-note"
          >
            {{ task.note }}
          </p>
          <p
            v-if="task.reminderTime"
            class="task-reminder-summary"
          >
            {{ task.reminderTime }} · {{ task.reminderContent || task.title }}
          </p>
        </div>
      </article>
      <p
        v-if="!daily.loading && (daily.day?.groups.length ?? 0) === 0"
        class="daily-empty"
      >
        还没有任务，先添加一项开始今天的工作。
      </p>
    </section>

    <Teleport to="body">
      <section
        v-if="createOpen && !daily.day?.settledAt"
        ref="createPopover"
        class="daily-create-popover"
        aria-label="新增当日任务"
        :style="createPopoverStyle"
        @click.stop
      >
        <header>
          <strong>新增当日任务</strong>
          <span>{{ daily.selectedDate }}</span>
        </header>
        <input
          v-model="draft.code"
          data-field="new-task-code"
          placeholder="编号，如 L36"
        >
        <input
          v-model="draft.title"
          data-field="new-task-title"
          placeholder="任务名称"
        >
        <label class="progress-control">
          <input
            v-model.number="draft.progress"
            class="task-progress-range"
            data-field="new-task-progress"
            min="0"
            max="100"
            type="range"
            :style="{ '--task-progress': `${draft.progress}%` }"
            title="进度百分比"
          >
          <output>{{ draft.progress }}%</output>
        </label>
        <textarea
          v-model="draft.note"
          data-field="new-task-note"
          placeholder="备注"
          rows="2"
        />
        <button
          class="daily-create-save"
          data-action="create-daily-task"
          type="button"
          @click="createTask"
        >
          创建任务
        </button>
      </section>

      <section
        v-if="reminderTask"
        ref="reminderPopover"
        class="task-reminder-popover"
        aria-label="任务提醒"
        :style="reminderPopoverStyle"
      >
        <header>
          <strong>任务提醒</strong>
          <span>{{ reminderTask.title }}</span>
        </header>
        <label>
          提醒时间
          <input
            v-model="reminderDraft.time"
            data-field="task-reminder-time"
            type="time"
          >
        </label>
        <label>
          提醒内容
          <textarea
            v-model="reminderDraft.content"
            data-field="task-reminder-content"
            rows="2"
          />
        </label>
        <button
          class="task-reminder-save"
          data-action="save-task-reminder"
          type="button"
          @click="saveReminder"
        >
          保存提醒
        </button>
      </section>
    </Teleport>

    <div
      v-if="editingTask"
      class="mask"
      @click.self="closeTaskEditor"
    >
      <section class="dialog task-editor-dialog">
        <header>
          <div>
            <p>Task editor</p>
            <h3>编辑任务</h3>
          </div>
          <button
            class="task-editor-close"
            type="button"
            title="关闭"
            aria-label="关闭"
            @click="closeTaskEditor"
          >
            <X :size="16" />
          </button>
        </header>
        <label>
          任务名称
          <input
            v-model="editDraft.title"
            data-field="task-editor-title"
          >
        </label>
        <label>
          备注
          <textarea
            v-model="editDraft.note"
            data-field="task-editor-note"
            rows="3"
          />
        </label>
        <label>
          提醒时间
          <input
            v-model="editDraft.reminderTime"
            data-field="task-editor-reminder-time"
            type="time"
          >
        </label>
        <label>
          提醒内容
          <textarea
            v-model="editDraft.reminderContent"
            data-field="task-editor-reminder-content"
            rows="2"
          />
        </label>
        <footer>
          <button
            type="button"
            @click="closeTaskEditor"
          >
            取消
          </button>
          <button
            class="task-editor-save"
            type="button"
            data-action="save-task-editor"
            @click="saveTaskEditor"
          >
            保存任务
          </button>
        </footer>
      </section>
    </div>
  </main>
</template>

<style scoped>
.daily-page { min-height: 100%; padding: 20px 22px; background: var(--bb-bg); }
.daily-toolbar,.date-nav,.daily-group header,.task-card-actions { display:flex; align-items:center; gap:8px; }
.daily-toolbar { justify-content:space-between; padding-bottom:16px; border-bottom:1px solid var(--bb-border); }
.daily-toolbar p,.daily-toolbar h2 { margin:0; }
.daily-toolbar p { color:var(--bb-primary); font:11px var(--bb-mono); letter-spacing:.08em; text-transform:uppercase; }
.daily-toolbar h2 { margin-top:3px; font-size:20px; }
.daily-title-row { display:flex; align-items:center; gap:8px; margin-top:3px; }
.daily-title-action { display:grid; width:24px; min-height:24px; place-items:center; padding:0; }
.date-nav button { display:grid; width:30px; min-height:30px; place-items:center; padding:0; }
.date-nav input { min-height:30px; padding:4px 8px; }
.daily-error,.daily-settled { margin-top:12px; padding:9px 11px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); color:var(--bb-text-muted); background:var(--bb-surface-soft); }
.daily-error { border-color:var(--bb-danger-border); color:#ffb6c0; background:var(--bb-danger-soft); }
.daily-task input { min-width:0; min-height:30px; padding:5px 8px; }
.daily-groups { display:grid; gap:16px; }
.daily-group { display:grid; gap:6px; }
.daily-group header { justify-content:space-between; padding:0 2px; color:var(--bb-primary-strong); }
.daily-group header span { color:var(--bb-text-soft); font:11px var(--bb-mono); }
.daily-task { position:relative; display:grid; gap:7px; padding:10px 84px 10px 10px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); background:rgba(12,23,33,.66); box-shadow:0 4px 14px rgba(0,0,0,.12); }
.task-card-main { display:grid; grid-template-columns:auto minmax(120px,1.25fr) minmax(130px,.9fr); align-items:center; gap:8px; min-width:0; }
.task-code { color:var(--bb-primary-strong); font:11px var(--bb-mono); white-space:nowrap; }
.task-title { width:100%; }
.progress-control { display:flex; align-items:center; min-width:0; gap:6px; color:var(--bb-text-soft); font:11px var(--bb-mono); }
.task-progress-range { --task-progress:0%; width:100%; min-width:70px; min-height:20px !important; padding:0 !important; appearance:none; background:linear-gradient(90deg, var(--bb-primary) 0 var(--task-progress), rgba(148,163,184,.2) var(--task-progress) 100%); border-radius:999px; cursor:pointer; }
.task-progress-range::-webkit-slider-runnable-track { height:4px; border-radius:999px; background:transparent; }
.task-progress-range::-webkit-slider-thumb { width:10px; height:10px; margin-top:-3px; appearance:none; border:2px solid var(--bb-bg); border-radius:50%; background:var(--bb-primary-strong); box-shadow:0 1px 4px rgba(0,0,0,.28); }
.task-progress-range:disabled { cursor:default; opacity:.6; }
.progress-control output { min-width:32px; color:var(--bb-text); text-align:right; }
.task-note,.task-reminder-summary { margin:0; color:var(--bb-text-soft); font-size:11px; line-height:1.45; overflow-wrap:anywhere; }
.task-reminder-summary { color:var(--bb-primary-strong); font-family:var(--bb-mono); }
.task-card-actions { position:absolute; top:10px; right:10px; justify-content:end; }
.task-reminder-button,.task-card-actions button { display:grid; width:26px; min-height:26px; place-items:center; padding:0; }
.task-reminder-button.active { border-color:rgba(102,247,211,.5); color:var(--bb-primary-strong); }
.task-card-actions button { color:#ff9aa8; }
.daily-empty { margin:32px 0; color:var(--bb-text-soft); text-align:center; }
.task-reminder-popover,.daily-create-popover { display:grid; gap:9px; max-height:min(420px, calc(100vh - 86px)); overflow:auto; padding:10px; border:1px solid var(--bb-border-strong); border-radius:var(--bb-radius-md); background:rgba(5,14,22,.98); box-shadow:var(--bb-shadow-floating); }
.task-reminder-popover header,.daily-create-popover header { display:grid; gap:3px; }
.task-reminder-popover strong,.daily-create-popover strong { font-size:13px; }
.task-reminder-popover header span,.daily-create-popover header span { overflow:hidden; color:var(--bb-text-soft); font-size:11px; text-overflow:ellipsis; white-space:nowrap; }
.task-reminder-popover label,.daily-create-popover label,.task-editor-dialog label { display:grid; gap:5px; color:var(--bb-text-soft); font-size:11px; }
.task-reminder-popover input,.task-reminder-popover textarea,.daily-create-popover input,.daily-create-popover textarea,.task-editor-dialog input,.task-editor-dialog textarea { width:100%; min-width:0; min-height:30px; padding:5px 8px; }
.task-reminder-save,.daily-create-save,.task-editor-save { border-color:rgba(102,247,211,.5); background:var(--bb-primary); color:#06231f; font-weight:750; }
.mask { position:fixed; inset:0; z-index:30; display:grid; place-items:center; padding:18px; background:rgba(1,5,9,.72); backdrop-filter:blur(6px); }
.dialog { width:min(420px,100%); display:grid; gap:12px; padding:16px; border:1px solid var(--bb-border-strong); border-radius:var(--bb-radius-md); background:var(--bb-surface); box-shadow:var(--bb-shadow-dialog); }
.task-editor-dialog header,.task-editor-dialog footer { display:flex; align-items:center; justify-content:space-between; gap:12px; }
.task-editor-dialog p,.task-editor-dialog h3 { margin:0; }
.task-editor-dialog p { color:var(--bb-primary); font-family:var(--bb-mono); font-size:10px; letter-spacing:.08em; text-transform:uppercase; }
.task-editor-dialog h3 { margin-top:3px; font-size:16px; }
.task-editor-close { display:grid; width:28px; min-height:28px; place-items:center; padding:0; }
.task-editor-dialog footer { justify-content:flex-end; }
@media (max-width:760px) { .daily-page { padding:14px 12px; } .daily-task { padding-right:76px; } .task-card-main { grid-template-columns:auto minmax(0,1fr); } .task-progress-control { grid-column:span 2; } .daily-toolbar { align-items:start; } }
</style>

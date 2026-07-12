<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { Check, ChevronLeft, ChevronRight, Copy, Plus, Save, Trash2 } from '@lucide/vue'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useUiStore } from '@/stores/ui'
import { getDailyReport } from '@/lib/productionIpc'
import { copyToClipboard } from '@/lib/ipc'

const daily = useDailyTasksStore()
const ui = useUiStore()
const draft = reactive({ code: '', title: '', progress: 0, note: '' })
const copyingReport = ref(false)
const reportCopied = ref(false)
let reportCopyResetTimer: ReturnType<typeof window.setTimeout> | null = null

function moveDate(days: number) {
  const [year, month, day] = daily.selectedDate.split('-').map(Number)
  const value = new Date(Date.UTC(year, month - 1, day + days))
  return daily.selectDate(value.toISOString().slice(0, 10))
}

async function createTask() {
  if (!draft.code.trim() || !draft.title.trim()) return
  await daily.create({ ...draft, code: draft.code.trim(), title: draft.title.trim(), investedMinutes: 0 })
  Object.assign(draft, { code: '', title: '', progress: 0, note: '' })
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

watch(() => daily.selectedDate, resetReportCopied)
onMounted(() => { void daily.selectDate(daily.selectedDate) })
onBeforeUnmount(resetReportCopied)
</script>

<template>
  <main class="daily-page">
    <header class="daily-toolbar">
      <div>
        <p>Daily ledger</p>
        <h2>当日任务</h2>
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

    <section
      v-if="!daily.day?.settledAt"
      class="daily-create"
    >
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
        rows="1"
      />
      <button
        data-action="create-daily-task"
        type="button"
        title="添加任务"
        aria-label="添加任务"
        @click="createTask"
      >
        <Plus :size="16" />
      </button>
    </section>

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
        >
          <div class="task-card-main">
            <span class="task-code">#{{ group.code }}</span>
            <input
              v-model="task.title"
              class="task-title"
              data-field="task-title"
              :disabled="Boolean(daily.day?.settledAt)"
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
              >
              <output data-progress-value>{{ task.progress }}%</output>
            </label>
            <span class="progress-hint">拖动调整完成进度百分比</span>
          </div>
          <textarea
            v-model="task.note"
            class="task-note"
            rows="1"
            placeholder="备注"
            :disabled="Boolean(daily.day?.settledAt)"
          />
          <div
            v-if="!daily.day?.settledAt"
            class="task-card-actions"
          >
            <button
              data-action="save-task"
              type="button"
              title="保存任务"
              aria-label="保存任务"
              @click="daily.update({ taskId: task.id, title: task.title, progress: task.progress, note: task.note, investedMinutes: task.investedMinutes })"
            >
              <Save :size="15" />
            </button>
            <button
              data-action="delete-task"
              type="button"
              title="删除任务"
              aria-label="删除任务"
              @click="daily.remove(task.id)"
            >
              <Trash2 :size="15" />
            </button>
          </div>
        </div>
      </article>
      <p
        v-if="!daily.loading && (daily.day?.groups.length ?? 0) === 0"
        class="daily-empty"
      >
        还没有任务，先添加一项开始今天的工作。
      </p>
    </section>
  </main>
</template>

<style scoped>
.daily-page { min-height: 100%; padding: 20px 22px; background: var(--bb-bg); }
.daily-toolbar,.date-nav,.daily-group header,.task-card-actions { display:flex; align-items:center; gap:8px; }
.daily-toolbar { justify-content:space-between; padding-bottom:16px; border-bottom:1px solid var(--bb-border); }
.daily-toolbar p,.daily-toolbar h2 { margin:0; }
.daily-toolbar p { color:var(--bb-primary); font:11px var(--bb-mono); letter-spacing:.08em; text-transform:uppercase; }
.daily-toolbar h2 { margin-top:3px; font-size:20px; }
.date-nav button,.task-actions button,.daily-create button { display:grid; width:30px; min-height:30px; place-items:center; padding:0; }
.date-nav input { min-height:30px; padding:4px 8px; }
.daily-error,.daily-settled { margin-top:12px; padding:9px 11px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); color:var(--bb-text-muted); background:var(--bb-surface-soft); }
.daily-error { border-color:var(--bb-danger-border); color:#ffb6c0; background:var(--bb-danger-soft); }
.daily-create { display:grid; grid-template-columns:90px minmax(160px,1fr) minmax(170px,.9fr) minmax(140px,1fr) 30px; align-items:center; gap:8px; margin:16px 0; }
.daily-create input,.daily-create textarea,.daily-task input,.daily-task textarea { min-width:0; min-height:30px; padding:5px 8px; }
.daily-create textarea,.task-note { resize:vertical; }
.daily-groups { display:grid; gap:16px; }
.daily-group { display:grid; gap:6px; }
.daily-group header { justify-content:space-between; padding:0 2px; color:var(--bb-primary-strong); }
.daily-group header span { color:var(--bb-text-soft); font:11px var(--bb-mono); }
.daily-task { position:relative; display:grid; gap:6px; padding:10px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); background:rgba(12,23,33,.66); box-shadow:0 4px 14px rgba(0,0,0,.12); }
.task-card-main { display:grid; grid-template-columns:auto minmax(120px,1.35fr) minmax(120px,.85fr) minmax(0,.7fr); align-items:center; gap:8px; min-width:0; padding-right:42px; }
.task-code { color:var(--bb-primary-strong); font:11px var(--bb-mono); white-space:nowrap; }
.task-title { width:100%; }
.progress-control { display:flex; align-items:center; min-width:0; gap:6px; color:var(--bb-text-soft); font:11px var(--bb-mono); }
.task-progress-range { --task-progress:0%; width:100%; min-width:70px; min-height:20px !important; padding:0 !important; appearance:none; background:linear-gradient(90deg, var(--bb-primary) 0 var(--task-progress), rgba(148,163,184,.2) var(--task-progress) 100%); border-radius:999px; cursor:pointer; }
.task-progress-range::-webkit-slider-runnable-track { height:4px; border-radius:999px; background:transparent; }
.task-progress-range::-webkit-slider-thumb { width:10px; height:10px; margin-top:-3px; appearance:none; border:2px solid var(--bb-bg); border-radius:50%; background:var(--bb-primary-strong); box-shadow:0 1px 4px rgba(0,0,0,.28); }
.task-progress-range:disabled { cursor:default; opacity:.6; }
.progress-control output { min-width:32px; color:var(--bb-text); text-align:right; }
.progress-hint { overflow:hidden; color:var(--bb-text-soft); font-size:10px; text-overflow:ellipsis; white-space:nowrap; }
.task-note { width:100%; color:var(--bb-text-soft); font-size:11px; }
.task-card-actions { position:absolute; top:7px; right:7px; opacity:.45; transition:opacity 150ms ease; }
.daily-task:hover .task-card-actions,.daily-task:focus-within .task-card-actions { opacity:1; }
.task-card-actions button { display:grid; width:26px; min-height:26px; place-items:center; padding:0; }
.task-card-actions button:last-child { color:#ff9aa8; }
.daily-empty { margin:32px 0; color:var(--bb-text-soft); text-align:center; }
@media (max-width:760px) { .daily-page { padding:14px 12px; } .daily-create { grid-template-columns:1fr 1fr; } .daily-create textarea,.daily-create button { grid-column:span 2; } .task-card-main { grid-template-columns:auto minmax(0,1fr); padding-right:38px; } .task-progress-control,.progress-hint { grid-column:span 2; } .daily-toolbar { align-items:start; } }
</style>

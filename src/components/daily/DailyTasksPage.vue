<script setup lang="ts">
import { onMounted, reactive } from 'vue'
import { ChevronLeft, ChevronRight, Plus, Save, Trash2 } from '@lucide/vue'
import { useDailyTasksStore } from '@/stores/dailyTasks'

const daily = useDailyTasksStore()
const draft = reactive({ code: '', title: '', progress: 0, note: '', investedMinutes: 0 })

function moveDate(days: number) {
  const [year, month, day] = daily.selectedDate.split('-').map(Number)
  const value = new Date(Date.UTC(year, month - 1, day + days))
  return daily.selectDate(value.toISOString().slice(0, 10))
}

async function createTask() {
  if (!draft.code.trim() || !draft.title.trim()) return
  await daily.create({ ...draft, code: draft.code.trim(), title: draft.title.trim() })
  Object.assign(draft, { code: '', title: '', progress: 0, note: '', investedMinutes: 0 })
}

onMounted(() => { void daily.selectDate(daily.selectedDate) })
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
      <input
        v-model.number="draft.progress"
        data-field="new-task-progress"
        min="0"
        max="100"
        type="number"
        title="进度百分比"
      >
      <input
        v-model.number="draft.investedMinutes"
        data-field="new-task-minutes"
        min="0"
        type="number"
        title="投入分钟"
      >
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
          class="daily-task"
          :data-task-id="task.id"
        >
          <input
            v-model="task.title"
            class="task-title"
            data-field="task-title"
            :disabled="Boolean(daily.day?.settledAt)"
          >
          <label><span>进度</span><input
            v-model.number="task.progress"
            data-field="task-progress"
            min="0"
            max="100"
            type="number"
            :disabled="Boolean(daily.day?.settledAt)"
          ></label>
          <label><span>分钟</span><input
            v-model.number="task.investedMinutes"
            min="0"
            type="number"
            :disabled="Boolean(daily.day?.settledAt)"
          ></label>
          <textarea
            v-model="task.note"
            class="task-note"
            rows="1"
            placeholder="备注"
            :disabled="Boolean(daily.day?.settledAt)"
          />
          <div
            v-if="!daily.day?.settledAt"
            class="task-actions"
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
.daily-toolbar,.date-nav,.daily-create,.daily-group header,.daily-task,.task-actions { display:flex; align-items:center; gap:8px; }
.daily-toolbar { justify-content:space-between; padding-bottom:16px; border-bottom:1px solid var(--bb-border); }
.daily-toolbar p,.daily-toolbar h2 { margin:0; }
.daily-toolbar p { color:var(--bb-primary); font:11px var(--bb-mono); letter-spacing:.08em; text-transform:uppercase; }
.daily-toolbar h2 { margin-top:3px; font-size:20px; }
.date-nav button,.task-actions button,.daily-create button { display:grid; width:30px; min-height:30px; place-items:center; padding:0; }
.date-nav input { min-height:30px; padding:4px 8px; }
.daily-error,.daily-settled { margin-top:12px; padding:9px 11px; border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); color:var(--bb-text-muted); background:var(--bb-surface-soft); }
.daily-error { border-color:var(--bb-danger-border); color:#ffb6c0; background:var(--bb-danger-soft); }
.daily-create { display:grid; grid-template-columns:90px minmax(160px,1fr) 72px 72px minmax(140px,1fr) 30px; margin:16px 0; }
.daily-create input,.daily-create textarea,.daily-task input,.daily-task textarea { min-width:0; min-height:30px; padding:5px 8px; }
.daily-create textarea,.task-note { resize:vertical; }
.daily-groups { display:grid; gap:12px; }
.daily-group { border:1px solid var(--bb-border); border-radius:var(--bb-radius-sm); background:rgba(12,23,33,.7); overflow:hidden; }
.daily-group header { justify-content:space-between; padding:9px 12px; background:rgba(102,247,211,.08); color:var(--bb-primary-strong); }
.daily-group header span { color:var(--bb-text-soft); font:11px var(--bb-mono); }
.daily-task { display:grid; grid-template-columns:minmax(140px,1.5fr) 70px 70px minmax(140px,1fr) 68px; padding:8px 10px; border-top:1px solid var(--bb-border); }
.daily-task label { display:grid; grid-template-columns:auto 1fr; align-items:center; gap:4px; color:var(--bb-text-soft); font-size:10px; }
.task-actions { justify-content:end; }
.task-actions button:last-child { color:#ff9aa8; }
.daily-empty { margin:32px 0; color:var(--bb-text-soft); text-align:center; }
@media (max-width:760px) { .daily-page { padding:14px 12px; } .daily-create,.daily-task { grid-template-columns:1fr 1fr; } .daily-create textarea,.daily-create button,.task-note,.task-actions { grid-column:span 2; } .daily-toolbar { align-items:start; } }
</style>

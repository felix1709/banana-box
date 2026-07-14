<script setup lang="ts">
import { computed } from 'vue'
import { STAGE_DEFINITIONS, type Project, type ProjectStage } from '@/domain/production'
import { useAuthStore } from '@/stores/auth'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'

const props = defineProps<{
  project: Project
}>()

const auth = useAuthStore()
const projects = useProjectsStore()
const workspaces = useWorkspacesStore()
const DAY = 24 * 60 * 60 * 1000

function dateValue(value: string) {
  return Date.parse(`${value}T00:00:00Z`)
}

function dateText(value: number) {
  return new Date(value).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', timeZone: 'UTC' })
}

const stages = computed(() =>
  STAGE_DEFINITIONS.map((definition) => ({
    definition,
    stage: props.project.stages.find((stage) => stage.stageKey === definition.key),
  })).filter(({ stage }) => stage && stage.progress < 100),
)

const timelineRange = computed(() => {
  const dates = stages.value.flatMap(({ stage }) =>
    stage ? [dateValue(stage.startDate), dateValue(stage.endDate)] : [],
  )
  const today = dateValue(new Date().toISOString().slice(0, 10))
  const start = Math.min(...dates, today)
  const end = Math.max(...dates, today) + DAY
  return { start, end, duration: Math.max(end - start, DAY) }
})

const axisLabels = computed(() => {
  const { start, duration } = timelineRange.value
  return [0, 0.5, 1].map((ratio) => ({
    ratio,
    label: dateText(start + duration * ratio),
  }))
})

function percentage(date: string) {
  const { start, duration } = timelineRange.value
  return Math.min(100, Math.max(0, ((dateValue(date) - start) / duration) * 100))
}

function stageStyle(stage: ProjectStage | undefined) {
  if (!stage) return { left: '0%', width: '0%' }
  const left = percentage(stage.startDate)
  const right = percentage(stage.endDate)
  return {
    left: `${left}%`,
    width: `${Math.max(right - left, 1.5)}%`,
  }
}

function markerStyle(stage: ProjectStage | undefined) {
  if (!stage) return { left: '0%' }
  return { left: `${stage.progress}%` }
}

const todayStyle = computed(() => ({
  left: `${percentage(new Date().toISOString().slice(0, 10))}%`,
}))

const actorName = computed(
  () => workspaces.profile?.displayName || auth.user?.email?.split('@')[0] || '',
)

async function updateStageProgress(stage: ProjectStage | undefined, progress: number) {
  if (!stage) return
  await projects.setStage({
    projectId: props.project.id,
    stageKey: stage.stageKey,
    startDate: stage.startDate,
    endDate: stage.endDate,
    progress,
    actorUserId: auth.user?.id ?? '',
    actorName: actorName.value,
  })
}
</script>

<template>
  <section
    class="project-timeline"
    :data-project-timeline="project.id"
  >
    <div class="timeline-heading">
      <div>
        <p>项目时间条</p>
        <h3>{{ project.code }} · {{ project.name }}</h3>
      </div>
      <span>{{ project.releaseDate }}</span>
    </div>

    <div
      class="timeline-axis"
      aria-hidden="true"
    >
      <span
        v-for="item in axisLabels"
        :key="item.ratio"
        :style="{ left: `${item.ratio * 100}%` }"
      >
        {{ item.label }}
      </span>
    </div>

    <div class="timeline-rows">
      <div
        class="today-line"
        data-today-line
        :style="todayStyle"
      >
        <span>今天</span>
      </div>
      <div
        v-for="item in stages"
        :key="item.definition.key"
        class="timeline-row"
      >
        <span class="timeline-stage-label">{{ item.definition.label }}</span>
        <div class="timeline-track">
          <div
            class="timeline-stage-bar"
            :data-stage-bar="item.definition.key"
            :style="{
              ...stageStyle(item.stage),
              '--stage-color': item.definition.color,
              '--stage-text': item.definition.textColor,
            }"
          >
            <span
              class="timeline-stage-progress"
              data-progress-marker
              :style="markerStyle(item.stage)"
            />
            <span
              v-if="item.stage"
              class="timeline-stage-value"
            >{{ item.stage.progress }}%</span>
          </div>
        </div>
        <label class="timeline-progress-editor">
          <input
            :value="item.stage?.progress ?? 0"
            type="range"
            min="0"
            max="100"
            step="1"
            data-field="project-stage-progress"
            :aria-label="`${item.definition.label}进度`"
            @change="updateStageProgress(item.stage, Number(($event.target as HTMLInputElement).value))"
          >
        </label>
      </div>
    </div>
  </section>
</template>

<style scoped>
.project-timeline {
  padding: 18px 20px 16px;
  border-top: 1px solid var(--bb-border);
  background: rgba(8, 19, 28, 0.66);
}

.timeline-heading {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.timeline-heading p,
.timeline-heading h3 {
  margin: 0;
}

.timeline-heading p {
  color: var(--bb-text-soft);
  font-size: 11px;
}

.timeline-heading h3 {
  margin-top: 3px;
  color: var(--bb-text);
  font-size: 15px;
  font-weight: 650;
}

.timeline-heading > span {
  color: var(--bb-text-muted);
  font-family: var(--bb-mono);
  font-size: 11px;
}

.timeline-axis {
  position: relative;
  height: 21px;
  margin-left: 66px;
  border-top: 1px solid rgba(148, 179, 188, 0.18);
}

.timeline-axis span {
  position: absolute;
  top: 4px;
  color: var(--bb-text-soft);
  font-size: 10px;
  transform: translateX(-50%);
}

.timeline-axis span:first-child {
  transform: none;
}

.timeline-axis span:last-child {
  transform: translateX(-100%);
}

.timeline-rows {
  position: relative;
}

.timeline-row {
  display: grid;
  grid-template-columns: 58px minmax(0, 1fr) 120px;
  gap: 8px;
  min-height: 24px;
  align-items: center;
}

.timeline-progress-editor input {
  width: 100%;
  min-height: 20px;
  padding: 0;
}

.timeline-stage-label {
  overflow: hidden;
  color: var(--bb-text-muted);
  font-size: 11px;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.timeline-track {
  position: relative;
  height: 13px;
  border-radius: 2px;
  background: rgba(148, 179, 188, 0.08);
}

.timeline-stage-bar {
  position: absolute;
  top: 1px;
  bottom: 1px;
  min-width: 8px;
  border-radius: 2px;
  background: var(--stage-color);
  color: var(--stage-text);
  box-shadow: 0 2px 8px color-mix(in srgb, var(--stage-color) 38%, transparent);
}

.timeline-stage-progress {
  position: absolute;
  top: -3px;
  width: 2px;
  height: 15px;
  border-radius: 2px;
  background: var(--stage-text);
  box-shadow: 0 0 0 1px rgba(5, 11, 18, 0.2);
  transform: translateX(-1px);
}

.timeline-stage-value {
  position: absolute;
  top: 0;
  right: 4px;
  color: inherit;
  font-family: var(--bb-mono);
  font-size: 9px;
  font-weight: 700;
  line-height: 11px;
}

.today-line {
  position: absolute;
  z-index: 3;
  top: -4px;
  bottom: -3px;
  width: 1px;
  background: rgba(234, 255, 249, 0.84);
  pointer-events: none;
}

.today-line span {
  position: absolute;
  top: -12px;
  left: 3px;
  color: var(--bb-primary-strong);
  font-size: 10px;
  white-space: nowrap;
}

@media (max-width: 720px) {
  .project-timeline {
    padding-inline: 12px;
  }

  .timeline-row {
    grid-template-columns: 58px minmax(0, 1fr);
  }

  .timeline-progress-editor {
    grid-column: 2;
  }
}
</style>

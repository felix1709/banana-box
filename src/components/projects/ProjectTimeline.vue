<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { STAGE_DEFINITIONS, type Project, type ProjectStage } from '@/domain/production'

const props = defineProps<{
  project: Project
}>()

const DAY = 24 * 60 * 60 * 1000
const scrollEl = ref<HTMLElement | null>(null)
let dragStartX = 0
let dragStartScrollLeft = 0
let dragging = false

function dateValue(value: string) {
  return Date.parse(`${value}T00:00:00Z`)
}

function localDate(value: number) {
  return new Date(value).toISOString().slice(0, 10)
}

function dateText(value: number) {
  return new Date(value).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', timeZone: 'UTC' })
}

function todayText(value: number) {
  const date = new Date(value)
  return `${date.getUTCMonth() + 1}.${date.getUTCDate()}`
}

const stageItems = computed(() =>
  STAGE_DEFINITIONS.map((definition) => ({
    definition,
    stage: props.project.stages.find((stage) => stage.stageKey === definition.key),
  })),
)

const projectRange = computed(() => {
  const dates = props.project.stages.flatMap((stage) => [dateValue(stage.startDate), dateValue(stage.endDate)])
  dates.push(dateValue(props.project.releaseDate))
  const start = Math.min(...dates)
  const end = Math.max(...dates)
  return { start, end, duration: Math.max(end - start, DAY) }
})

const displayToday = computed(() => {
  const today = dateValue(new Date().toISOString().slice(0, 10))
  return Math.min(today, projectRange.value.end)
})

const axisLabels = computed(() => {
  const { start, duration } = projectRange.value
  return [
    { ratio: 0, label: dateText(start), boundary: 'start' },
    { ratio: 0.5, label: dateText(start + duration * 0.5), boundary: '' },
    { ratio: 1, label: dateText(dateValue(props.project.releaseDate)), boundary: 'release' },
  ]
})

function percentage(date: string) {
  const { start, duration } = projectRange.value
  return Math.min(100, Math.max(0, ((dateValue(date) - start) / duration) * 100))
}

function todayPercentage() {
  const { start, duration } = projectRange.value
  return Math.min(100, Math.max(0, ((displayToday.value - start) / duration) * 100))
}

function scheduleProgress(stage: ProjectStage) {
  const start = dateValue(stage.startDate)
  const end = dateValue(stage.endDate)
  if (displayToday.value <= start) return 0
  if (displayToday.value >= end) return 100
  return Math.round(((displayToday.value - start) / Math.max(end - start, DAY)) * 100)
}

function stageStyle(stage: ProjectStage | undefined) {
  if (!stage || scheduleProgress(stage) === 0) return null
  const left = percentage(stage.startDate)
  const endDate = localDate(Math.min(displayToday.value, dateValue(stage.endDate)))
  const right = percentage(endDate)
  return {
    left: `${left}%`,
    width: `${Math.max(right - left, 1.5)}%`,
  }
}

const todayStyle = computed(() => ({
  left: `${todayPercentage()}%`,
}))

function onTimelineMouseDown(event: MouseEvent) {
  if (!scrollEl.value) return
  dragging = true
  dragStartX = event.clientX
  dragStartScrollLeft = scrollEl.value.scrollLeft
  window.addEventListener('mousemove', onTimelineMouseMove)
  window.addEventListener('mouseup', stopTimelineDrag)
}

function onTimelineMouseMove(event: MouseEvent) {
  if (!dragging || !scrollEl.value) return
  scrollEl.value.scrollLeft = dragStartScrollLeft + dragStartX - event.clientX
}

function stopTimelineDrag() {
  dragging = false
  window.removeEventListener('mousemove', onTimelineMouseMove)
  window.removeEventListener('mouseup', stopTimelineDrag)
}

onBeforeUnmount(stopTimelineDrag)
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
      ref="scrollEl"
      class="timeline-scroll"
      data-timeline-scroll
      @mousedown="onTimelineMouseDown"
    >
      <div class="timeline-canvas">
        <div
          class="timeline-axis"
          aria-hidden="true"
        >
          <span
            v-for="item in axisLabels"
            :key="item.ratio"
            :class="{ boundary: item.boundary }"
            :data-axis-boundary="item.boundary || undefined"
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
            <span>{{ todayText(displayToday) }}</span>
          </div>
          <div
            v-for="item in stageItems"
            :key="item.definition.key"
            class="timeline-row"
            data-stage-row
          >
            <span class="timeline-stage-label">{{ item.definition.label }}</span>
            <div class="timeline-track">
              <div
                v-if="stageStyle(item.stage)"
                class="timeline-stage-bar"
                :data-stage-bar="item.definition.key"
                :style="{
                  ...stageStyle(item.stage),
                  '--stage-color': item.definition.color,
                  '--stage-text': item.definition.textColor,
                }"
              >
                <span
                  v-if="item.stage"
                  class="timeline-stage-value"
                >{{ scheduleProgress(item.stage) }}%</span>
              </div>
            </div>
          </div>
        </div>
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

.timeline-scroll {
  overflow-x: auto;
  cursor: grab;
  user-select: none;
}

.timeline-scroll:active {
  cursor: grabbing;
}

.timeline-canvas {
  min-width: 760px;
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
  display: inline-block;
  font-size: 10px;
  line-height: 1;
  white-space: nowrap;
  transform: translateX(-50%);
}

.timeline-axis span.boundary {
  color: #f4c430;
  font-weight: 750;
}

.timeline-axis span:first-child {
  transform: none;
}

.timeline-axis span:last-child {
  right: 0;
  left: auto !important;
  transform: translateX(-100%);
}

.timeline-rows {
  position: relative;
}

.timeline-row {
  display: grid;
  grid-template-columns: 58px minmax(0, 1fr);
  gap: 8px;
  min-height: 24px;
  align-items: center;
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
  background: #ff5b67;
  pointer-events: none;
}

.today-line span {
  position: absolute;
  top: -13px;
  left: 4px;
  color: #ff9aa8;
  font-family: var(--bb-mono);
  font-size: 10px;
  white-space: nowrap;
}

@media (max-width: 720px) {
  .project-timeline {
    padding-inline: 12px;
  }

  .timeline-canvas {
    min-width: 680px;
  }
}
</style>

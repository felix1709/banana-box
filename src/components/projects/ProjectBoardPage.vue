<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Pencil, Plus } from '@lucide/vue'
import { STAGE_DEFINITIONS, type StageKey } from '@/domain/production'
import { useProjectsStore } from '@/stores/projects'
import ProjectTimeline from './ProjectTimeline.vue'

const projects = useProjectsStore()
const selectedProjectId = ref<string | null>(null)

const selectedProject = computed(
  () =>
    projects.filteredProjects.find((project) => project.id === selectedProjectId.value) ??
    projects.filteredProjects[0] ??
    null,
)

const timelineProjects = computed(() =>
  projects.filteredProjects.filter((project) => project.stages.some((stage) => stage.progress < 100)),
)

watch(
  () => projects.filteredProjects.map((project) => project.id),
  (projectIds) => {
    if (!selectedProjectId.value || !projectIds.includes(selectedProjectId.value)) {
      selectedProjectId.value = projectIds[0] ?? null
    }
  },
  { immediate: true },
)

function selectStage(stageKey: StageKey | 'all') {
  projects.filters.stageKey = stageKey
}

function openNewProject() {
  projects.openEditor(null)
}

function openSelectedProject() {
  if (selectedProject.value) projects.openEditor(selectedProject.value.id)
}
</script>

<template>
  <main class="project-board-page">
    <header class="project-board-toolbar">
      <div>
        <p>Production</p>
        <h2>项目管理</h2>
      </div>

      <div class="project-board-controls">
        <input
          v-model="projects.filters.query"
          aria-label="搜索项目"
          placeholder="搜索编号或项目名称"
        >
        <select
          v-model="projects.filters.stageKey"
          aria-label="筛选项目阶段"
        >
          <option value="all">
            全部阶段
          </option>
          <option
            v-for="stage in STAGE_DEFINITIONS"
            :key="stage.key"
            :value="stage.key"
          >
            {{ stage.label }}
          </option>
        </select>
        <input
          v-model="projects.filters.releaseDate"
          aria-label="筛选发布时间"
          type="date"
        >
        <button
          v-if="selectedProject"
          class="project-toolbar-icon"
          data-action="edit-selected-project"
          type="button"
          title="编辑当前项目"
          aria-label="编辑当前项目"
          @click="openSelectedProject"
        >
          <Pencil
            :size="15"
            aria-hidden="true"
          />
        </button>
        <button
          class="new-project-button"
          type="button"
          @click="openNewProject"
        >
          <Plus
            :size="15"
            :stroke-width="2.4"
            aria-hidden="true"
          />
          新建项目
        </button>
      </div>
    </header>

    <div
      v-if="projects.error"
      class="project-board-error"
      role="alert"
    >
      {{ projects.error }}
    </div>

    <div class="project-board-scroll">
      <section
        class="project-board"
        aria-label="项目阶段看板"
      >
        <section
          v-for="stage in STAGE_DEFINITIONS"
          :key="stage.key"
          class="project-stage-column"
          :data-stage-column="stage.key"
        >
          <header :style="{ '--stage-color': stage.color, '--stage-text-color': stage.textColor }">
            <button
              type="button"
              @click="selectStage(stage.key)"
            >
              {{ stage.label }}
            </button>
            <span>{{ projects.projectsByStage[stage.key].length }}</span>
          </header>

          <div class="project-stage-list">
            <button
              v-for="project in projects.projectsByStage[stage.key]"
              :key="project.id"
              class="project-stage-card"
              :class="{ selected: selectedProject?.id === project.id }"
              :data-project-id="project.id"
              type="button"
              @click="selectedProjectId = project.id"
            >
              <span class="project-code">{{ project.code }} · {{ project.version }}</span>
              <strong>{{ project.name }}</strong>
              <span class="project-release">{{ project.releaseDate }}</span>
              <span class="project-progress">
                <span :style="{ width: `${project.stages.find((item) => item.stageKey === stage.key)?.progress ?? 0}%` }" />
              </span>
              <b>{{ project.stages.find((item) => item.stageKey === stage.key)?.progress ?? 0 }}%</b>
            </button>
            <p v-if="projects.projectsByStage[stage.key].length === 0">
              暂无项目
            </p>
          </div>
        </section>
      </section>
      <section class="project-timeline-list">
        <ProjectTimeline
          v-for="project in timelineProjects"
          :key="project.id"
          :project="project"
        />
      </section>
    </div>
  </main>
</template>

<style scoped>
.project-board-page {
  display: flex;
  min-height: 0;
  height: 100%;
  flex-direction: column;
  background: var(--bb-bg);
}

.project-board-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 22px 16px;
  border-bottom: 1px solid var(--bb-border);
}

.project-board-toolbar p,
.project-board-toolbar h2 {
  margin: 0;
}

.project-board-toolbar p {
  color: var(--bb-primary);
  font-family: var(--bb-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.project-board-toolbar h2 {
  margin-top: 3px;
  font-size: 20px;
  font-weight: 650;
}

.project-board-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.project-board-controls input,
.project-board-controls select {
  height: 30px;
  padding: 0 9px;
}

.project-board-controls input[aria-label='搜索项目'] {
  width: 170px;
}

.new-project-button {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary);
  color: #06231f;
  font-weight: 700;
}

.project-toolbar-icon {
  display: grid;
  width: 30px;
  min-height: 30px;
  place-items: center;
  padding: 0;
}

.new-project-button:hover:not(:disabled) {
  background: var(--bb-primary-strong);
}

.project-board-error {
  padding: 8px 22px;
  border-bottom: 1px solid var(--bb-danger-border);
  background: var(--bb-danger-soft);
  color: #ffb6c0;
}

.project-board-scroll {
  min-height: 0;
  flex: 1;
  overflow: auto;
}

.project-timeline-list {
  border-top: 1px solid var(--bb-border);
}

.project-board {
  display: grid;
  min-width: 1260px;
  grid-template-columns: repeat(8, minmax(148px, 1fr));
  align-items: stretch;
  min-height: 100%;
}

.project-stage-column {
  min-width: 0;
  border-right: 1px solid var(--bb-border);
  background: rgba(12, 23, 33, 0.38);
}

.project-stage-column:last-child {
  border-right: 0;
}

.project-stage-column > header {
  display: flex;
  height: 38px;
  align-items: center;
  justify-content: space-between;
  padding: 0 9px;
  border-bottom: 1px solid rgba(5, 11, 18, 0.18);
  background: var(--stage-color);
  color: var(--stage-text-color);
}

.project-stage-column > header button {
  min-height: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font-weight: 750;
}

.project-stage-column > header button:hover:not(:disabled) {
  box-shadow: none;
  text-decoration: underline;
}

.project-stage-column > header span {
  display: grid;
  width: 18px;
  height: 18px;
  place-items: center;
  border-radius: 50%;
  background: color-mix(in srgb, var(--stage-text-color) 16%, transparent);
  color: inherit;
  font-family: var(--bb-mono);
  font-size: 10px;
  font-weight: 700;
}

.project-stage-list {
  display: grid;
  align-content: start;
  gap: 8px;
  min-height: 132px;
  padding: 10px;
}

.project-stage-list > p {
  margin: 5px 0;
  color: var(--bb-text-soft);
  font-size: 11px;
  text-align: center;
}

.project-stage-card {
  display: grid;
  min-width: 0;
  gap: 6px;
  padding: 10px;
  border-color: rgba(148, 179, 188, 0.18);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-surface-soft);
  color: var(--bb-text);
  text-align: left;
}

.project-stage-card.selected {
  border-color: var(--bb-primary);
  box-shadow: 0 0 0 1px rgba(102, 247, 211, 0.14), 0 10px 18px rgba(0, 0, 0, 0.14);
}

.project-code,
.project-release,
.project-stage-card b {
  color: var(--bb-text-muted);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.project-stage-card strong {
  overflow: hidden;
  color: var(--bb-text);
  font-size: 13px;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-release {
  color: var(--bb-text-soft);
}

.project-progress {
  display: block;
  height: 4px;
  overflow: hidden;
  border-radius: 2px;
  background: rgba(148, 179, 188, 0.14);
}

.project-progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--bb-primary);
}

.project-stage-card b {
  color: var(--bb-primary-strong);
  font-weight: 700;
}

@media (max-width: 760px) {
  .project-board-toolbar {
    padding: 16px 12px 12px;
  }

  .project-board-controls input[aria-label='搜索项目'] {
    width: min(100%, 220px);
  }
}
</style>

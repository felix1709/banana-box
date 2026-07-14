<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Pencil, Plus, Users, UserPlus } from '@lucide/vue'
import { STAGE_DEFINITIONS, type Project, type StageKey } from '@/domain/production'
import { useAuthStore } from '@/stores/auth'
import { useProjectsStore } from '@/stores/projects'
import InviteDialog from '@/components/collaboration/InviteDialog.vue'
import CommentPanel from '@/components/collaboration/CommentPanel.vue'
import PresenceAvatars from '@/components/collaboration/PresenceAvatars.vue'
import ProjectTimeline from './ProjectTimeline.vue'

const auth = useAuthStore()
const projects = useProjectsStore()
const selectedProjectId = ref<string | null>(null)
const detailOpen = ref(false)
const inviteOpen = ref(false)
const inviteTrigger = ref<HTMLButtonElement | null>(null)
const invitePopover = ref<HTMLElement | null>(null)
const invitePopoverStyle = ref<Record<string, string>>({})

const selectedProject = computed(
  () =>
    projects.filteredProjects.find((project) => project.id === selectedProjectId.value) ??
    projects.filteredProjects[0] ??
    null,
)
const selectedProjectOwnedByCurrentUser = computed(
  () => Boolean(auth.user && selectedProject.value?.ownerUserId === auth.user.id),
)
const selectedProjectCanEditMetadata = computed(
  () => Boolean(selectedProject.value && (!auth.user || !selectedProject.value.ownerUserId || selectedProjectOwnedByCurrentUser.value)),
)
const selectedProjectCanInvite = computed(
  () => Boolean(selectedProjectOwnedByCurrentUser.value && selectedProject.value?.isPublic),
)

watch(
  () => projects.filteredProjects.map((project) => project.id),
  (projectIds) => {
    if (!selectedProjectId.value || !projectIds.includes(selectedProjectId.value)) {
      selectedProjectId.value = projectIds[0] ?? null
      detailOpen.value = false
    }
  },
  { immediate: true },
)

function stageDefinition(stageKey: StageKey) {
  return STAGE_DEFINITIONS.find((stage) => stage.key === stageKey) ?? STAGE_DEFINITIONS[0]
}

function selectProject(project: Project) {
  selectedProjectId.value = project.id
  detailOpen.value = true
}

function openNewProject() {
  projects.openEditor(null)
}

function openSelectedProject() {
  if (selectedProject.value) projects.openEditor(selectedProject.value.id)
}

function editProject(project: Project) {
  if (auth.user && project.ownerUserId && project.ownerUserId !== auth.user.id) return
  selectedProjectId.value = project.id
  projects.openEditor(project.id)
}

async function positionInvitePopover() {
  await nextTick()
  const rect = inviteTrigger.value?.getBoundingClientRect()
  if (!rect) return
  const width = Math.min(340, window.innerWidth - 24)
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, rect.right - width))
  invitePopoverStyle.value = {
    position: 'fixed',
    zIndex: '240',
    top: `${rect.bottom + 8}px`,
    left: `${left}px`,
    width: `${width}px`,
  }
}

async function toggleInvitePopover() {
  inviteOpen.value = !inviteOpen.value
  if (inviteOpen.value) await positionInvitePopover()
}

async function toggleSelectedProjectPublic() {
  if (!selectedProject.value || !selectedProjectOwnedByCurrentUser.value) return
  await projects.setPublic(selectedProject.value.id, !selectedProject.value.isPublic)
}

function closeInviteWhenClickingOutside(event: MouseEvent) {
  if (!inviteOpen.value) return
  const target = event.target
  if (!(target instanceof Node)) return
  if (inviteTrigger.value?.contains(target)) return
  if (invitePopover.value?.contains(target)) return
  inviteOpen.value = false
}

onMounted(() => {
  document.addEventListener('click', closeInviteWhenClickingOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', closeInviteWhenClickingOutside)
})
</script>

<template>
  <main class="project-board-page">
    <header class="project-board-toolbar">
      <div class="project-board-title">
        <p>Production</p>
        <div class="project-title-row">
          <h2>项目管理</h2>
          <button
            v-if="auth.user && selectedProjectCanInvite"
            ref="inviteTrigger"
            class="project-toolbar-icon"
            data-action="project-invite-menu"
            type="button"
            title="协作邀请"
            aria-label="协作邀请"
            :aria-expanded="inviteOpen"
            @click="toggleInvitePopover"
          >
            <UserPlus
              :size="16"
              aria-hidden="true"
            />
          </button>
        </div>
      </div>

      <div class="project-board-controls">
        <PresenceAvatars v-if="auth.user" />
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
          v-if="selectedProject && selectedProjectCanEditMetadata"
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
          v-if="selectedProject && selectedProjectOwnedByCurrentUser"
          class="project-public-toggle"
          type="button"
          data-action="toggle-project-public"
          :aria-pressed="selectedProject.isPublic"
          @click="toggleSelectedProjectPublic"
        >
          <Users
            :size="15"
            aria-hidden="true"
          />
          {{ selectedProject.isPublic ? '公共项目' : '个人项目' }}
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

    <Teleport to="body">
      <section
        v-if="auth.user && inviteOpen && selectedProjectCanInvite"
        ref="invitePopover"
        class="project-invite-popover"
        aria-label="协作邀请"
        :style="invitePopoverStyle"
      >
        <header>
          <strong>协作邀请</strong>
          <span>{{ selectedProject ? selectedProject.code : '全部项目' }}</span>
        </header>
        <InviteDialog
          :project-id="selectedProject?.id ?? null"
          :can-invite="selectedProjectCanInvite"
        />
      </section>
    </Teleport>

    <div
      v-if="projects.error"
      class="project-board-error"
      role="alert"
    >
      {{ projects.error }}
    </div>

    <div class="project-board-scroll">
      <section
        class="project-note-grid"
        aria-label="项目便签"
      >
        <button
          v-for="project in projects.filteredProjects"
          :key="project.id"
          class="project-note"
          :class="{ selected: selectedProject?.id === project.id && detailOpen }"
          :data-project-note="project.id"
          type="button"
          @click="selectProject(project)"
          @dblclick.stop="editProject(project)"
        >
          <span
            v-if="project.isPublic"
            class="project-note-public-badge"
            :data-project-public-badge="project.id"
            title="公共协作项目"
            aria-label="公共协作项目"
          >
            <Users
              :size="13"
              aria-hidden="true"
            />
          </span>
          <span
            class="project-note-stage"
            :style="{
              '--stage-color': stageDefinition(project.mainStageKey).color,
              '--stage-text-color': stageDefinition(project.mainStageKey).textColor,
            }"
          >
            {{ stageDefinition(project.mainStageKey).label }}
          </span>
          <span class="project-note-code">{{ project.code }}</span>
          <span class="project-note-version">{{ project.version }}</span>
          <strong>{{ project.name }}</strong>
          <span class="project-note-release">{{ project.releaseDate }}</span>
          <span
            v-if="project.lastActivitySummary"
            class="project-note-activity"
          >
            {{ project.lastActivityActorName ? `${project.lastActivityActorName}：` : '' }}{{ project.lastActivitySummary }}
          </span>
        </button>
        <p
          v-if="!projects.loading && projects.filteredProjects.length === 0"
          class="project-note-empty"
        >
          暂无项目
        </p>
      </section>

      <section
        v-if="detailOpen && selectedProject"
        class="project-detail-panel"
        aria-label="项目具体排期进度"
      >
        <ProjectTimeline :project="selectedProject" />
        <CommentPanel
          v-if="auth.user"
          target-type="project"
          :target-id="selectedProject.id"
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

.project-board-title p,
.project-board-title h2 {
  margin: 0;
}

.project-board-title p {
  color: var(--bb-primary);
  font-family: var(--bb-mono);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.project-title-row {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  margin-top: 3px;
}

.project-board-title h2 {
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

.project-public-toggle {
  display: inline-flex;
  min-height: 30px;
  align-items: center;
  gap: 6px;
  padding: 0 9px;
  font-size: 12px;
}

.new-project-button:hover:not(:disabled) {
  background: var(--bb-primary-strong);
}

.project-invite-popover {
  position: fixed;
  z-index: 240;
  max-height: min(460px, calc(100vh - 86px));
  overflow: auto;
  display: grid;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.98);
  box-shadow: var(--bb-shadow-floating);
}

.project-invite-popover > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.project-invite-popover strong {
  font-size: 13px;
}

.project-invite-popover span {
  overflow: hidden;
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.project-note-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(174px, 1fr));
  gap: 10px;
  align-items: stretch;
  padding: 14px;
}

.project-note {
  position: relative;
  display: grid;
  min-width: 0;
  min-height: 132px;
  align-content: start;
  gap: 7px;
  padding: 11px;
  border: 1px solid rgba(148, 179, 188, 0.18);
  border-radius: var(--bb-radius-sm);
  background:
    linear-gradient(180deg, rgba(20, 35, 47, 0.96), rgba(9, 21, 31, 0.96));
  color: var(--bb-text);
  text-align: left;
  box-shadow: var(--bb-shadow-card);
}

.project-note-public-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  display: grid;
  width: 24px;
  min-height: 24px;
  place-items: center;
  border: 1px solid rgba(102, 247, 211, 0.66);
  border-radius: var(--bb-radius-xs);
  background: rgba(102, 247, 211, 0.16);
  color: var(--bb-primary-strong);
}

.project-note:hover,
.project-note.selected {
  border-color: rgba(123, 255, 226, 0.42);
  background:
    radial-gradient(circle at 100% 0%, rgba(102, 247, 211, 0.1), transparent 38%),
    linear-gradient(180deg, rgba(23, 41, 54, 0.98), rgba(10, 22, 32, 0.98));
  box-shadow: var(--bb-shadow-floating);
}

.project-note-stage {
  justify-self: start;
  max-width: 100%;
  padding: 3px 7px;
  border-radius: var(--bb-radius-xs);
  background: var(--stage-color);
  color: var(--stage-text-color);
  font-size: 11px;
  font-weight: 750;
}

.project-note-code,
.project-note-version,
.project-note-release {
  color: var(--bb-text-muted);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.project-note strong {
  min-width: 0;
  overflow: hidden;
  color: var(--bb-text);
  font-size: 14px;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-note-release {
  margin-top: auto;
  color: var(--bb-text-soft);
}

.project-note-activity {
  overflow: hidden;
  color: var(--bb-text-muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-note-empty {
  grid-column: 1 / -1;
  margin: 32px 0;
  color: var(--bb-text-soft);
  text-align: center;
}

.project-detail-panel {
  border-top: 1px solid var(--bb-border);
}

@media (max-width: 760px) {
  .project-board-toolbar {
    padding: 16px 12px 12px;
  }

  .project-board-controls input[aria-label='搜索项目'] {
    width: min(100%, 220px);
  }

  .project-note-grid {
    grid-template-columns: repeat(auto-fit, minmax(146px, 1fr));
    padding: 10px;
  }
}
</style>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Pencil, Plus, Users, UserPlus } from '@lucide/vue'
import { STAGE_DEFINITIONS, type Project, type StageKey } from '@/domain/production'
import { useAuthStore } from '@/stores/auth'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'
import InviteDialog from '@/components/collaboration/InviteDialog.vue'
import CommentPanel from '@/components/collaboration/CommentPanel.vue'
import PresenceAvatars from '@/components/collaboration/PresenceAvatars.vue'
import ProjectTimeline from './ProjectTimeline.vue'

const auth = useAuthStore()
const projects = useProjectsStore()
const workspaces = useWorkspacesStore()
const selectedProjectId = ref<string | null>(null)
const detailOpen = ref(false)
const inviteOpen = ref(false)
const inviteTrigger = ref<HTMLButtonElement | null>(null)
const invitePopover = ref<HTMLElement | null>(null)
const invitePopoverStyle = ref<Record<string, string>>({})
const logOpen = ref(false)
const logTrigger = ref<HTMLButtonElement | null>(null)
const logPopover = ref<HTMLElement | null>(null)
const logPopoverStyle = ref<Record<string, string>>({})

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
const projectLogEntries = computed(() =>
  projects.filteredProjects
    .filter((project) => project.lastActivitySummary)
    .map((project) => ({
      id: project.id,
      code: project.code,
      name: project.name,
      actorName: project.lastActivityActorName,
      summary: project.lastActivitySummary,
    })),
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

function projectOwnedByCurrentUser(project: Project) {
  return Boolean(auth.user && project.ownerUserId === auth.user.id)
}

function projectCanInvite(project: Project) {
  return projectOwnedByCurrentUser(project)
}

async function openProjectInvite(project: Project, event: MouseEvent) {
  if (!projectCanInvite(project)) return
  if (!auth.client || !auth.user || !workspaces.activeWorkspaceId) {
    projects.error = '请先登录并完成云端同步配置'
    return
  }
  selectedProjectId.value = project.id
  inviteTrigger.value = event.currentTarget as HTMLButtonElement
  if (!project.isPublic) {
    try {
      await projects.setPublic(project.id, true)
    } catch (error) {
      projects.error = error instanceof Error ? error.message : String(error)
      return
    }
  }
  try {
    await projects.ensureCloudProject(auth.client, workspaces.activeWorkspaceId, auth.user.id, project.id)
  } catch (error) {
    projects.error = error instanceof Error ? error.message : String(error)
    return
  }
  inviteOpen.value = true
  await positionInvitePopover()
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

async function positionLogPopover() {
  await nextTick()
  const rect = logTrigger.value?.getBoundingClientRect()
  if (!rect) return
  const width = Math.min(360, window.innerWidth - 24)
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, rect.left))
  logPopoverStyle.value = {
    position: 'fixed',
    zIndex: '240',
    top: `${rect.bottom + 8}px`,
    left: `${left}px`,
    width: `${width}px`,
  }
}

async function toggleLogPopover() {
  logOpen.value = !logOpen.value
  if (logOpen.value) await positionLogPopover()
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

function closeLogWhenClickingOutside(event: MouseEvent) {
  if (!logOpen.value) return
  const target = event.target
  if (!(target instanceof Node)) return
  if (logTrigger.value?.contains(target)) return
  if (logPopover.value?.contains(target)) return
  logOpen.value = false
}

onMounted(() => {
  document.addEventListener('click', closeInviteWhenClickingOutside)
  document.addEventListener('click', closeLogWhenClickingOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', closeInviteWhenClickingOutside)
  document.removeEventListener('click', closeLogWhenClickingOutside)
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
            ref="logTrigger"
            class="project-log-trigger"
            data-action="project-log-menu"
            type="button"
            :aria-expanded="logOpen"
            @click="toggleLogPopover"
          >
            项目日志
          </button>
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
        v-if="logOpen"
        ref="logPopover"
        class="project-log-popover"
        aria-label="项目日志"
        :style="logPopoverStyle"
        @click.stop
      >
        <header>
          <strong>项目日志</strong>
          <span>{{ projectLogEntries.length }} 条</span>
        </header>
        <ul v-if="projectLogEntries.length">
          <li
            v-for="entry in projectLogEntries"
            :key="entry.id"
          >
            <small>{{ entry.code }} · {{ entry.name }}</small>
            <span>{{ entry.actorName ? `${entry.actorName}：` : '' }}{{ entry.summary }}</span>
          </li>
        </ul>
        <p v-else>
          暂无修改记录
        </p>
      </section>

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
        <article
          v-for="project in projects.filteredProjects"
          :key="project.id"
          class="project-note"
          :class="{ selected: selectedProject?.id === project.id && detailOpen }"
          :data-project-note="project.id"
          role="button"
          tabindex="0"
          @click="selectProject(project)"
          @keydown.enter.prevent="selectProject(project)"
          @keydown.space.prevent="selectProject(project)"
          @dblclick.stop="editProject(project)"
        >
          <span class="project-note-badges">
            <span
              v-if="auth.user && project.ownerUserId"
              class="project-note-owner-badge"
              :class="{ collaborator: !projectOwnedByCurrentUser(project) }"
              :data-project-owner-badge="project.id"
              :title="projectOwnedByCurrentUser(project) ? '项目发起人，最高权限' : '项目协作成员'"
            >
              {{ projectOwnedByCurrentUser(project) ? '发起人' : '协作' }}
            </span>
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
          <button
            v-if="projectCanInvite(project)"
            class="project-note-invite"
            data-action="card-project-invite"
            :data-project-invite="project.id"
            type="button"
            title="添加协作用户"
            aria-label="添加协作用户"
            @click.stop="openProjectInvite(project, $event)"
          >
            <UserPlus
              :size="13"
              aria-hidden="true"
            />
            添加协作用户
          </button>
        </article>
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

.project-log-trigger {
  min-height: 22px;
  padding: 0 7px;
  border-color: rgba(123, 255, 226, 0.18);
  background: rgba(4, 12, 18, 0.34);
  color: var(--bb-text-soft);
  font-size: 11px;
}

.project-log-trigger:hover:not(:disabled),
.project-log-trigger[aria-expanded='true'] {
  border-color: rgba(123, 255, 226, 0.42);
  color: var(--bb-primary);
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

.project-log-popover {
  position: fixed;
  z-index: 240;
  max-height: min(360px, calc(100vh - 86px));
  overflow: auto;
  display: grid;
  gap: 9px;
  padding: 10px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.98);
  box-shadow: var(--bb-shadow-floating);
}

.project-log-popover > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.project-log-popover strong {
  font-size: 13px;
}

.project-log-popover header span,
.project-log-popover small {
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 10px;
}

.project-log-popover ul {
  display: grid;
  gap: 7px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.project-log-popover li {
  display: grid;
  gap: 3px;
  min-width: 0;
  padding: 7px;
  border: 1px solid rgba(123, 255, 226, 0.12);
  border-radius: var(--bb-radius-sm);
  background: rgba(102, 247, 211, 0.05);
}

.project-log-popover li span {
  overflow-wrap: anywhere;
  color: var(--bb-text);
  font-size: 12px;
  line-height: 1.45;
}

.project-log-popover p {
  margin: 2px 0;
  color: var(--bb-text-soft);
  font-size: 12px;
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
  cursor: pointer;
  font: inherit;
  text-align: left;
  box-shadow: var(--bb-shadow-card);
}

.project-note:focus-visible {
  outline: 2px solid rgba(123, 255, 226, 0.72);
  outline-offset: 2px;
}

.project-note-badges {
  position: absolute;
  top: 8px;
  right: 8px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.project-note-public-badge {
  display: grid;
  width: 24px;
  min-height: 24px;
  place-items: center;
  border: 1px solid rgba(102, 247, 211, 0.66);
  border-radius: var(--bb-radius-xs);
  background: rgba(102, 247, 211, 0.16);
  color: var(--bb-primary-strong);
}

.project-note-owner-badge {
  display: inline-flex;
  min-height: 24px;
  align-items: center;
  padding: 0 7px;
  border: 1px solid rgba(255, 206, 84, 0.48);
  border-radius: var(--bb-radius-xs);
  background: rgba(255, 206, 84, 0.14);
  color: #ffd978;
  font-size: 10px;
  font-weight: 750;
  white-space: nowrap;
}

.project-note-owner-badge.collaborator {
  border-color: rgba(123, 255, 226, 0.28);
  background: rgba(123, 255, 226, 0.08);
  color: var(--bb-text-soft);
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

.project-note-invite {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  justify-content: center;
  gap: 5px;
  margin-top: 2px;
  padding: 0 8px;
  border-color: rgba(123, 255, 226, 0.28);
  background: rgba(123, 255, 226, 0.08);
  color: var(--bb-primary);
  font-size: 11px;
  font-weight: 650;
}

.project-note-invite:hover:not(:disabled),
.project-note-invite:focus-visible {
  border-color: rgba(123, 255, 226, 0.52);
  background: rgba(123, 255, 226, 0.14);
  color: var(--bb-primary-strong);
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

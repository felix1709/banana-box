<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { X } from '@lucide/vue'
import {
  STAGE_DEFINITIONS,
  type Project,
  type ProjectStageInput,
  type SaveProjectWithStagesInput,
} from '@/domain/production'
import { useAuthStore } from '@/stores/auth'
import { useMembersStore } from '@/stores/members'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'

const projects = useProjectsStore()
const auth = useAuthStore()
const members = useMembersStore()
const workspaces = useWorkspacesStore()
const saving = ref(false)
const error = ref('')

type EditorStage = ProjectStageInput

interface EditorForm {
  code: string
  version: string
  name: string
  filePath: string
  releaseDate: string
  archived: boolean
  shareMode: 'personal' | 'public'
  inviteIdentity: string
  stages: EditorStage[]
}

function today() {
  return new Date().toISOString().slice(0, 10)
}

function initialStages(project: Project | null): EditorStage[] {
  return STAGE_DEFINITIONS.map((definition) => {
    const stage = project?.stages.find((item) => item.stageKey === definition.key)
    return {
      stageKey: definition.key,
      startDate: stage?.startDate ?? today(),
      endDate: stage?.endDate ?? today(),
      progress: stage?.progress ?? 0,
    }
  })
}

function formFromProject(project: Project | null): EditorForm {
  return {
    code: project?.code ?? '',
    version: project?.version ?? '',
    name: project?.name ?? '',
    filePath: project?.filePath ?? '',
    releaseDate: project?.releaseDate ?? today(),
    archived: project?.archived ?? false,
    shareMode: project?.isPublic ? 'public' : 'personal',
    inviteIdentity: '',
    stages: initialStages(project),
  }
}

const form = reactive<EditorForm>(formFromProject(projects.editingProject))

watch(
  () => projects.editingProject,
  (project) => Object.assign(form, formFromProject(project)),
  { immediate: true },
)

function close() {
  if (!saving.value) projects.closeEditor()
}

async function save() {
  saving.value = true
  error.value = ''
  try {
    let savedProject: Project
    if (projects.editingProject) {
      const input: SaveProjectWithStagesInput = {
        projectId: projects.editingProject.id,
        code: form.code.trim(),
        version: form.version.trim(),
        name: form.name.trim(),
        filePath: form.filePath.trim(),
        releaseDate: form.releaseDate,
        archived: form.archived,
        stages: form.stages.map(({ stageKey, startDate, endDate }) => ({
          stageKey,
          startDate,
          endDate,
          progress: 0,
        })),
      }
      savedProject = await projects.saveEditor(input)
    } else {
      savedProject = await projects.create({
        code: form.code.trim(),
        version: form.version.trim(),
        name: form.name.trim(),
        filePath: form.filePath.trim(),
        releaseDate: form.releaseDate,
        ownerUserId: auth.user?.id ?? '',
        stages: form.stages.map(({ stageKey, startDate, endDate }) => ({
          stageKey,
          startDate,
          endDate,
          progress: 0,
        })),
      })
    }
    if (form.shareMode === 'public') {
      if (!auth.client || !auth.user || !workspaces.activeWorkspaceId) {
        throw new Error('请先登录云端账号，再创建公共项目')
      }
      if (!savedProject.isPublic) {
        savedProject = await projects.setPublic(savedProject.id, true)
      }
      await projects.ensureCloudProject(auth.client, workspaces.activeWorkspaceId, auth.user.id, savedProject.id)

      const identity = form.inviteIdentity.trim()
      if (identity) {
        const recipient = await members.resolveInviteRecipient(auth.client, identity)
        const invite = await members.createInvite(auth.client, {
          appOrigin: 'banana-box://invite',
          workspaceId: workspaces.activeWorkspaceId,
          projectId: savedProject.id,
          scopeType: 'project',
          role: 'editor',
          email: recipient.email,
          userId: auth.user.id,
        })
        const notificationResponse = await auth.client.from('notifications').insert({
          workspace_id: workspaces.activeWorkspaceId,
          recipient_user_id: recipient.id,
          actor_user_id: auth.user.id,
          kind: 'invite',
          target_type: 'project_invite',
          target_id: invite.id,
          created_by: auth.user.id,
          updated_by: auth.user.id,
        })
        if (notificationResponse.error) throw new Error(notificationResponse.error.message)
      }
    }
    projects.closeEditor()
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div
    class="project-editor-mask"
    role="presentation"
    @mousedown.self="close"
  >
    <form
      class="project-editor"
      aria-label="项目编辑器"
      @submit.prevent="save"
    >
      <header>
        <div>
          <p>{{ projects.editingProject ? 'Edit production' : 'New production' }}</p>
          <h2>{{ projects.editingProject ? '编辑项目' : '新建项目' }}</h2>
        </div>
        <button
          class="project-editor-close"
          type="button"
          title="关闭项目编辑器"
          aria-label="关闭项目编辑器"
          @click="close"
        >
          <X
            :size="18"
            aria-hidden="true"
          />
        </button>
      </header>

      <div class="project-editor-scroll">
        <div
          v-if="error"
          class="project-editor-error"
          role="alert"
        >
          {{ error }}
        </div>

        <section
          class="project-fields"
          aria-label="项目信息"
        >
          <label>
            <span>项目编号</span>
            <input
              v-model="form.code"
              data-field="code"
              required
              maxlength="32"
              placeholder="例如 L36"
            >
          </label>
          <label>
            <span>项目版本</span>
            <input
              v-model="form.version"
              data-field="version"
              required
              maxlength="64"
              placeholder="例如 v1"
            >
          </label>
          <label class="wide">
            <span>项目名称</span>
            <input
              v-model="form.name"
              data-field="name"
              required
              maxlength="800"
              placeholder="输入项目名称"
            >
          </label>
          <label class="wide">
            <span>文件地址</span>
            <input
              v-model="form.filePath"
              data-field="file-path"
              required
              maxlength="32768"
              placeholder="C:\\work\\project"
            >
          </label>
          <label>
            <span>发布时间</span>
            <input
              v-model="form.releaseDate"
              data-field="release-date"
              required
              type="date"
            >
          </label>
          <fieldset class="project-share-fields wide">
            <legend>项目权限</legend>
            <label>
              <input
                v-model="form.shareMode"
                data-field="share-personal"
                type="radio"
                value="personal"
              >
              <span>个人项目</span>
            </label>
            <label>
              <input
                v-model="form.shareMode"
                data-field="share-public"
                type="radio"
                value="public"
              >
              <span>公共项目</span>
            </label>
            <label
              v-if="form.shareMode === 'public'"
              class="project-invite-field"
            >
              <span>邀请协作用户</span>
              <input
                v-model="form.inviteIdentity"
                data-field="invite-identity"
                placeholder="000002 或昵称"
              >
            </label>
          </fieldset>
          <label
            v-if="projects.editingProject"
            class="project-archive-toggle"
          >
            <input
              v-model="form.archived"
              type="checkbox"
            >
            <span>归档项目</span>
          </label>
        </section>

        <section
          class="project-stage-editor"
          aria-label="项目阶段排期"
        >
          <div class="project-stage-editor-heading">
            <div>
              <p>Stage schedule</p>
              <h3>阶段排期</h3>
            </div>
            <span>阶段可以重叠安排</span>
          </div>

          <div class="project-stage-grid">
            <div
              v-for="(stage, index) in form.stages"
              :key="stage.stageKey"
              class="project-stage-row"
            >
              <span
                class="project-stage-name"
                :style="{
                  '--stage-color': STAGE_DEFINITIONS[index].color,
                  '--stage-text': STAGE_DEFINITIONS[index].textColor,
                }"
              >
                {{ STAGE_DEFINITIONS[index].label }}
              </span>
              <label>
                <span>开始</span>
                <input
                  v-model="stage.startDate"
                  required
                  type="date"
                >
              </label>
              <label>
                <span>结束</span>
                <input
                  v-model="stage.endDate"
                  required
                  type="date"
                >
              </label>
            </div>
          </div>
        </section>
      </div>

      <footer>
        <button
          type="button"
          @click="close"
        >
          取消
        </button>
        <button
          class="project-editor-save"
          type="submit"
          :disabled="saving"
        >
          {{ saving ? '保存中…' : '保存项目' }}
        </button>
      </footer>
    </form>
  </div>
</template>

<style scoped>
.project-editor-mask {
  position: fixed;
  z-index: 40;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(1, 5, 9, 0.76);
  backdrop-filter: blur(8px);
}

.project-editor {
  display: flex;
  width: min(820px, 100%);
  max-height: min(820px, calc(100vh - 40px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-dialog);
}

.project-editor > header,
.project-editor > footer {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 20px;
  border-bottom: 1px solid var(--bb-border);
}

.project-editor > header p,
.project-editor > header h2,
.project-stage-editor-heading p,
.project-stage-editor-heading h3 {
  margin: 0;
}

.project-editor > header p,
.project-stage-editor-heading p {
  color: var(--bb-primary);
  font-family: var(--bb-mono);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.project-editor > header h2 {
  margin-top: 3px;
  font-size: 18px;
}

.project-editor-close {
  display: grid;
  width: 30px;
  min-height: 30px;
  place-items: center;
  padding: 0;
}

.project-editor-scroll {
  min-height: 0;
  overflow: auto;
  padding: 20px;
}

.project-editor-error {
  margin-bottom: 14px;
  padding: 8px 10px;
  border: 1px solid var(--bb-danger-border);
  border-radius: var(--bb-radius-sm);
  background: var(--bb-danger-soft);
  color: #ffb6c0;
}

.project-fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.project-fields label,
.project-stage-row label {
  display: grid;
  gap: 5px;
  min-width: 0;
}

.project-fields label > span,
.project-stage-row label > span {
  color: var(--bb-text-muted);
  font-size: 11px;
}

.project-fields input,
.project-stage-row input {
  width: 100%;
  min-height: 31px;
  padding: 4px 8px;
}

.project-fields .wide {
  grid-column: span 2;
}

.project-share-fields {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: 8px 12px;
  margin: 2px 0 0;
  padding: 10px;
  border: 1px solid rgba(148, 179, 188, 0.16);
  border-radius: var(--bb-radius-sm);
}

.project-share-fields legend {
  padding: 0 4px;
  color: var(--bb-text-muted);
  font-size: 11px;
}

.project-share-fields label {
  display: inline-flex;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 6px;
}

.project-share-fields input[type='radio'] {
  width: auto;
  min-height: 0;
}

.project-invite-field {
  display: grid !important;
  min-width: min(260px, 100%);
  flex: 1;
}

.project-archive-toggle {
  display: inline-flex !important;
  grid-template-columns: auto 1fr;
  align-items: center;
  justify-self: start;
  gap: 7px !important;
}

.project-archive-toggle input {
  width: auto;
  min-height: 0;
}

.project-stage-editor {
  margin-top: 24px;
  padding-top: 18px;
  border-top: 1px solid var(--bb-border);
}

.project-stage-editor-heading {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 12px;
}

.project-stage-editor-heading h3 {
  margin-top: 3px;
  font-size: 15px;
}

.project-stage-editor-heading > span {
  color: var(--bb-text-soft);
  font-size: 11px;
}

.project-stage-grid {
  display: grid;
  border-top: 1px solid rgba(148, 179, 188, 0.14);
}

.project-stage-row {
  display: grid;
  grid-template-columns: 72px minmax(132px, 1fr) minmax(132px, 1fr);
  gap: 10px;
  align-items: end;
  padding: 10px 0;
  border-bottom: 1px solid rgba(148, 179, 188, 0.12);
}

.project-stage-name {
  align-self: center;
  padding: 4px 7px;
  border-radius: 3px;
  background: var(--stage-color);
  color: var(--stage-text);
  font-size: 11px;
  font-weight: 700;
  text-align: center;
}

.project-editor > footer {
  justify-content: flex-end;
  border-top: 1px solid var(--bb-border);
  border-bottom: 0;
}

.project-editor > footer button {
  min-height: 30px;
  padding: 0 12px;
}

.project-editor-save {
  border-color: rgba(102, 247, 211, 0.5);
  background: var(--bb-primary);
  color: #06231f;
  font-weight: 750;
}

.project-editor-save:hover:not(:disabled) {
  background: var(--bb-primary-strong);
}

@media (max-width: 680px) {
  .project-editor-mask {
    padding: 0;
  }

  .project-editor {
    width: 100%;
    height: 100%;
    max-height: none;
    border: 0;
    border-radius: 0;
  }

  .project-editor-scroll,
  .project-editor > header,
  .project-editor > footer {
    padding-inline: 14px;
  }

  .project-stage-row {
    grid-template-columns: 64px minmax(0, 1fr) minmax(0, 1fr);
  }
}
</style>

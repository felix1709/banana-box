<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { X } from '@lucide/vue'
import {
  STAGE_DEFINITIONS,
  type Project,
  type ProjectStageInput,
  type SaveProjectWithStagesInput,
  type StageKey,
} from '@/domain/production'
import { useProjectsStore } from '@/stores/projects'

const projects = useProjectsStore()
const saving = ref(false)
const error = ref('')

type EditorStage = ProjectStageInput

interface EditorForm {
  code: string
  version: string
  name: string
  filePath: string
  releaseDate: string
  mainStageKey: StageKey
  archived: boolean
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
    mainStageKey: project?.mainStageKey ?? 'storyboard',
    archived: project?.archived ?? false,
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
    if (projects.editingProject) {
      const input: SaveProjectWithStagesInput = {
        projectId: projects.editingProject.id,
        code: form.code.trim(),
        version: form.version.trim(),
        name: form.name.trim(),
        filePath: form.filePath.trim(),
        releaseDate: form.releaseDate,
        mainStageKey: form.mainStageKey,
        archived: form.archived,
        stages: form.stages.map((stage) => ({ ...stage, progress: Number(stage.progress) })),
      }
      await projects.saveEditor(input)
    } else {
      await projects.create({
        code: form.code.trim(),
        version: form.version.trim(),
        name: form.name.trim(),
        filePath: form.filePath.trim(),
        releaseDate: form.releaseDate,
        mainStageKey: form.mainStageKey,
        stages: form.stages.map((stage) => ({ ...stage, progress: Number(stage.progress) })),
      })
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
          <label>
            <span>当前主阶段</span>
            <select v-model="form.mainStageKey">
              <option
                v-for="stage in STAGE_DEFINITIONS"
                :key="stage.key"
                :value="stage.key"
              >
                {{ stage.label }}
              </option>
            </select>
          </label>
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
              <label class="stage-progress-field">
                <span>进度</span>
                <input
                  v-model.number="stage.progress"
                  :data-stage-progress="stage.stageKey"
                  required
                  min="0"
                  max="100"
                  step="1"
                  type="number"
                >
                <i>%</i>
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
.project-fields select,
.project-stage-row input {
  width: 100%;
  min-height: 31px;
  padding: 4px 8px;
}

.project-fields .wide {
  grid-column: span 2;
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
  grid-template-columns: 72px minmax(132px, 1fr) minmax(132px, 1fr) 82px;
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

.stage-progress-field {
  position: relative;
}

.stage-progress-field input {
  padding-right: 20px;
  font-family: var(--bb-mono);
}

.stage-progress-field i {
  position: absolute;
  right: 7px;
  bottom: 8px;
  color: var(--bb-text-soft);
  font-family: var(--bb-mono);
  font-size: 11px;
  font-style: normal;
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

  .stage-progress-field {
    grid-column: 2 / -1;
  }
}
</style>

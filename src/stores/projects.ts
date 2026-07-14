import { defineStore } from 'pinia'
import {
  STAGE_DEFINITIONS,
  type CreateProjectInput,
  type Project,
  type ProjectFilter,
  type SaveProjectWithStagesInput,
  type SetProjectStageInput,
  type StageKey,
} from '@/domain/production'
import {
  archiveProject,
  createProject,
  deleteProject,
  listProjects,
  saveProjectWithStages,
  setProjectPublic,
  setProjectStage,
} from '@/lib/productionIpc'

const emptyFilters = (): ProjectFilter => ({
  query: '',
  stageKey: 'all',
  releaseDate: '',
  archived: false,
})

function sortProjects(projects: Project[]) {
  return [...projects].sort((left, right) => {
    const releaseOrder = left.releaseDate.localeCompare(right.releaseDate)
    if (releaseOrder !== 0) return releaseOrder
    const codeOrder = left.code.localeCompare(right.code, undefined, { sensitivity: 'accent' })
    if (codeOrder !== 0) return codeOrder
    return left.id.localeCompare(right.id)
  })
}

function stageColumns(): Record<StageKey, Project[]> {
  return STAGE_DEFINITIONS.reduce(
    (columns, stage) => {
      columns[stage.key] = []
      return columns
    },
    {} as Record<StageKey, Project[]>,
  )
}

export const useProjectsStore = defineStore('projects', {
  state: () => ({
    projects: [] as Project[],
    filters: emptyFilters(),
    loading: false,
    error: '' as string,
    editorProjectId: null as string | null,
    projectEditorOpen: false,
  }),

  getters: {
    filteredProjects(state): Project[] {
      const query = state.filters.query.trim().toLocaleLowerCase()
      return sortProjects(
        state.projects.filter((project) => {
          const matchesQuery =
            !query ||
            project.code.toLocaleLowerCase().includes(query) ||
            project.name.toLocaleLowerCase().includes(query)
          const matchesStage =
            state.filters.stageKey === 'all' || project.mainStageKey === state.filters.stageKey
          const matchesRelease =
            !state.filters.releaseDate || project.releaseDate === state.filters.releaseDate
          const matchesArchive =
            state.filters.archived === 'all' || project.archived === state.filters.archived

          return matchesQuery && matchesStage && matchesRelease && matchesArchive
        }),
      )
    },

    projectsByStage(): Record<StageKey, Project[]> {
      const columns = stageColumns()
      for (const project of this.filteredProjects) {
        columns[project.mainStageKey].push(project)
      }
      return columns
    },

    editingProject(state): Project | null {
      return state.projects.find((project) => project.id === state.editorProjectId) ?? null
    },
  },

  actions: {
    async load() {
      this.loading = true
      this.error = ''
      try {
        this.hydrate(await listProjects())
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.loading = false
      }
    },

    hydrate(projects: Project[]) {
      this.projects = sortProjects(projects)
    },

    async saveEditor(input: SaveProjectWithStagesInput) {
      const project = await saveProjectWithStages(input)
      this.replaceProject(project)
      return project
    },

    async create(input: CreateProjectInput) {
      const project = await createProject(input)
      this.replaceProject(project)
      return project
    },

    async setStage(input: SetProjectStageInput) {
      const project = await setProjectStage(input)
      this.replaceProject(project)
      return project
    },

    async setPublic(projectId: string, isPublic: boolean) {
      const project = await setProjectPublic(projectId, isPublic)
      this.replaceProject(project)
      return project
    },

    async setArchived(projectId: string, archived: boolean) {
      const project = await archiveProject(projectId, archived)
      this.replaceProject(project)
      return project
    },

    async remove(projectId: string) {
      await deleteProject(projectId)
      this.projects = this.projects.filter((project) => project.id !== projectId)
      if (this.editorProjectId === projectId) this.editorProjectId = null
    },

    replaceProject(project: Project) {
      const index = this.projects.findIndex((item) => item.id === project.id)
      if (index === -1) this.projects.push(project)
      else this.projects.splice(index, 1, project)
      this.projects = sortProjects(this.projects)
    },

    openEditor(projectId: string | null) {
      this.editorProjectId = projectId
      this.projectEditorOpen = true
    },

    closeEditor() {
      this.projectEditorOpen = false
      this.editorProjectId = null
    },
  },
})

import { invoke } from '@tauri-apps/api/core'
import type {
  CreateProjectInput,
  Project,
  SaveProjectWithStagesInput,
  SetProjectStageInput,
  UpdateProjectInput,
} from '@/domain/production'

export const listProjects = () => invoke<Project[]>('list_projects', {})

export const createProject = (input: CreateProjectInput) =>
  invoke<Project>('create_project', { input })

export const updateProject = (input: UpdateProjectInput) =>
  invoke<Project>('update_project', { input })

export const saveProjectWithStages = (input: SaveProjectWithStagesInput) =>
  invoke<Project>('save_project_with_stages', { input })

export const setProjectStage = (input: SetProjectStageInput) =>
  invoke<Project>('set_project_stage', { input })

export const archiveProject = (projectId: string, archived: boolean) =>
  invoke<Project>('archive_project', { projectId, archived })

export const deleteProject = (projectId: string) =>
  invoke<void>('delete_project', { projectId })

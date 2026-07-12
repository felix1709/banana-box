import { invoke } from '@tauri-apps/api/core'
import type {
  CreateProjectInput,
  CreateDailyTaskInput,
  DailyTaskDay,
  Project,
  SaveProjectWithStagesInput,
  SetProjectStageInput,
  UpdateProjectInput,
  UpdateDailyTaskInput,
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

export const loadDailyTaskDay = (localDate: string) =>
  invoke<DailyTaskDay>('load_daily_task_day', { localDate })

export const createDailyTask = (input: CreateDailyTaskInput) =>
  invoke<DailyTaskDay>('create_daily_task', { input })

export const updateDailyTask = (input: UpdateDailyTaskInput) =>
  invoke<DailyTaskDay>('update_daily_task', { input })

export const deleteDailyTask = (localDate: string, taskId: string) =>
  invoke<DailyTaskDay>('delete_daily_task', { localDate, taskId })

export const reorderDailyGroups = (localDate: string, groupIds: string[]) =>
  invoke<DailyTaskDay>('reorder_daily_groups', { input: { localDate, groupIds } })

export const reorderDailyTasks = (localDate: string, groupId: string, taskIds: string[]) =>
  invoke<DailyTaskDay>('reorder_daily_tasks', { input: { localDate, groupId, taskIds } })

import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import { STAGE_DEFINITIONS, type Project, type ProjectStage, type StageKey } from '@/domain/production'
import { useProjectsStore } from '@/stores/projects'

type SyncClient = Pick<SupabaseClient, 'from'>
type SyncState = 'idle' | 'syncing' | 'synced' | 'error' | 'conflict'

interface CloudProjectRow {
  id: string
  code: string
  version: string
  name: string
  file_display_ref: string | null
  release_date: string
  main_stage_key: StageKey
  archived: boolean
  owner_user_id: string | null
  is_public: boolean
  last_activity_summary: string | null
  last_activity_actor_name: string | null
  created_at: string
  updated_at: string
}

interface CloudProjectStageRow {
  id: string
  project_id: string
  stage_key: StageKey
  position: number
  start_date: string
  end_date: string
  progress: number
  updated_at: string
}

const SYNC_TABLES = [
  'prompt_categories',
  'prompts',
  'projects',
  'project_stages',
  'daily_task_days',
  'daily_task_groups',
  'daily_tasks',
  'comments',
  'project_activity_log',
] as const

function cloudStageToProjectStage(row: CloudProjectStageRow): ProjectStage {
  return {
    id: row.id,
    stageKey: row.stage_key,
    position: row.position,
    startDate: row.start_date,
    endDate: row.end_date,
    progress: row.progress,
    updatedAt: row.updated_at,
  }
}

function fallbackStages(project: CloudProjectRow): ProjectStage[] {
  return STAGE_DEFINITIONS.map((stage, position) => ({
    id: `${project.id}-${stage.key}`,
    stageKey: stage.key,
    position,
    startDate: project.release_date,
    endDate: project.release_date,
    progress: 0,
    updatedAt: project.updated_at,
  }))
}

function cloudProjectToProject(row: CloudProjectRow, stages: ProjectStage[]): Project {
  return {
    id: row.id,
    code: row.code,
    version: row.version,
    name: row.name,
    filePath: row.file_display_ref ?? '',
    fileExists: true,
    releaseDate: row.release_date,
    mainStageKey: row.main_stage_key,
    archived: row.archived,
    ownerUserId: row.owner_user_id ?? '',
    isPublic: row.is_public,
    lastActivitySummary: row.last_activity_summary ?? '',
    lastActivityActorName: row.last_activity_actor_name ?? '',
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    stages,
  }
}

function mapCloudProjects(rowsByTable: Record<string, unknown[]>): Project[] {
  const projectRows = (rowsByTable.projects ?? []) as CloudProjectRow[]
  const stageRows = (rowsByTable.project_stages ?? []) as CloudProjectStageRow[]
  const stagesByProject = new Map<string, ProjectStage[]>()

  for (const row of stageRows) {
    const stages = stagesByProject.get(row.project_id) ?? []
    stages.push(cloudStageToProjectStage(row))
    stagesByProject.set(row.project_id, stages)
  }

  return projectRows.map((project) => {
    const stages = (stagesByProject.get(project.id) ?? fallbackStages(project))
      .sort((left, right) => left.position - right.position)
    return cloudProjectToProject(project, stages)
  })
}

export const useSyncStatusStore = defineStore('syncStatus', {
  state: () => ({
    state: 'idle' as SyncState,
    lastSyncedAt: '',
    error: '',
    pendingOutbox: 0,
    conflicts: [] as string[],
    snapshots: {} as Record<string, unknown[]>,
  }),
  actions: {
    async pullWorkspace(client: SyncClient, workspaceId: string) {
      this.state = 'syncing'
      this.error = ''

      for (const table of SYNC_TABLES) {
        const response = await client
          .from(table)
          .select('*')
          .eq('workspace_id', workspaceId)

        if (response.error) {
          this.state = 'error'
          this.error = response.error.message
          return
        }
        this.snapshots[table] = response.data ?? []
      }

      const projects = mapCloudProjects(this.snapshots)
      if (projects.length > 0) useProjectsStore().hydrate(projects)

      this.lastSyncedAt = new Date().toISOString()
      this.state = this.conflicts.length > 0 ? 'conflict' : 'synced'
    },
    markConflict(recordId: string) {
      if (!this.conflicts.includes(recordId)) this.conflicts.push(recordId)
      this.state = 'conflict'
    },
    clearError() {
      this.error = ''
      if (this.state === 'error') this.state = 'idle'
    },
  },
})

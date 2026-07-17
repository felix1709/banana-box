import { defineStore } from 'pinia'
import { useAuthStore } from '@/stores/auth'
import { useDailyTasksStore } from '@/stores/dailyTasks'
import { useLibraryStore } from '@/stores/library'
import { useProjectsStore } from '@/stores/projects'
import { useWorkspacesStore } from '@/stores/workspaces'

type MigrationStatus = 'idle' | 'running' | 'completed' | 'skipped' | 'error'
type SummaryKey = 'categories' | 'prompts' | 'projects' | 'dailyDays' | 'dailyTasks'

interface CloudSummary {
  categories: number
  prompts: number
  projects: number
  dailyDays: number
  dailyTasks: number
  loaded: boolean
  loading: boolean
}

interface LocalSummary {
  categories: number
  prompts: number
  projects: number
  dailyDays: number
  dailyTasks: number
  hasLocalData: boolean
}

interface ComparisonRow {
  key: SummaryKey
  label: string
  local: number
  cloud: number
  pendingUpload: number
}

const EMPTY_CLOUD_SUMMARY: CloudSummary = {
  categories: 0,
  prompts: 0,
  projects: 0,
  dailyDays: 0,
  dailyTasks: 0,
  loaded: false,
  loading: false,
}

const COMPARISON_ROWS: Array<{ key: SummaryKey, label: string, table: string }> = [
  { key: 'categories', label: '分类', table: 'prompt_categories' },
  { key: 'prompts', label: '提示词', table: 'prompts' },
  { key: 'projects', label: '项目', table: 'projects' },
  { key: 'dailyDays', label: '每日计划', table: 'daily_task_days' },
  { key: 'dailyTasks', label: '任务', table: 'daily_tasks' },
]

function isoFromSeconds(seconds: number) {
  return new Date(seconds * 1000).toISOString()
}

export const useCloudMigrationStore = defineStore('cloudMigration', {
  state: () => ({
    status: 'idle' as MigrationStatus,
    error: '',
    dismissed: false,
    cloudSummary: { ...EMPTY_CLOUD_SUMMARY },
  }),
  getters: {
    summary(): LocalSummary {
      const library = useLibraryStore()
      const projects = useProjectsStore()
      const daily = useDailyTasksStore()
      const dailyTasks = daily.day?.groups.reduce((count, group) => count + group.tasks.length, 0) ?? 0
      const summary = {
        categories: library.library.categories.length,
        prompts: library.library.prompts.length,
        projects: projects.projects.length,
        dailyDays: daily.day ? 1 : 0,
        dailyTasks,
        hasLocalData: false,
      }
      summary.hasLocalData =
        summary.categories + summary.prompts + summary.projects + summary.dailyDays + summary.dailyTasks > 0
      return summary
    },
    comparison(state): ComparisonRow[] {
      const summary = this.summary as LocalSummary
      return COMPARISON_ROWS.map((row) => {
        const local = summary[row.key]
        const cloud = state.cloudSummary[row.key]
        return {
          key: row.key,
          label: row.label,
          local,
          cloud,
          pendingUpload: Math.max(local - cloud, 0),
        }
      })
    },
    shouldPrompt(): boolean {
      const auth = useAuthStore()
      return Boolean(
        auth.user
        && this.summary.hasLocalData
        && !['completed', 'skipped'].includes(this.status)
        && !this.dismissed,
      )
    },
  },
  actions: {
    decideLater() {
      this.dismissed = true
    },
    keepLocal() {
      this.status = 'skipped'
      this.dismissed = true
    },
    async loadCloudSummary() {
      const auth = useAuthStore()
      const workspaces = useWorkspacesStore()
      if (!auth.client || !workspaces.activeWorkspaceId || this.cloudSummary.loading) return

      const client = auth.client
      this.cloudSummary.loading = true
      this.error = ''
      try {
        const entries = await Promise.all(
          COMPARISON_ROWS.map(async (row) => {
            const response = await client
              .from(row.table)
              .select('id', { count: 'exact', head: true })
              .eq('workspace_id', workspaces.activeWorkspaceId)
              .is('deleted_at', null)
            if (response.error) throw new Error(response.error.message)
            return [row.key, response.count ?? 0] as const
          }),
        )
        this.cloudSummary = {
          ...EMPTY_CLOUD_SUMMARY,
          ...Object.fromEntries(entries),
          loaded: true,
          loading: false,
        }
      } catch (error) {
        this.cloudSummary.loading = false
        this.error = error instanceof Error ? error.message : String(error)
      }
    },
    async migrateNow() {
      const auth = useAuthStore()
      const workspaces = useWorkspacesStore()
      if (!auth.client || !auth.user || !workspaces.activeWorkspaceId) {
        this.error = '请先登录并选择一个云端工作区'
        this.status = 'error'
        return
      }

      this.status = 'running'
      this.error = ''
      const workspaceId = workspaces.activeWorkspaceId
      const userId = auth.user.id
      const library = useLibraryStore()
      const projects = useProjectsStore()
      const daily = useDailyTasksStore()
      const personalPrompts = library.library.prompts.filter(
        (prompt) => prompt.sourceType !== 'shared' && !prompt.sharedPromptId,
      )

      try {
        if (library.library.categories.length > 0) {
          const response = await auth.client.from('prompt_categories').upsert(
            library.library.categories.map((category) => ({
              id: category.id,
              workspace_id: workspaceId,
              name: category.name,
              color: category.color,
              position: category.order,
              created_by: userId,
              updated_by: userId,
            })),
          )
          if (response.error) throw new Error(response.error.message)
        }

        if (personalPrompts.length > 0) {
          const promptResponse = await auth.client.from('prompts').upsert(
            personalPrompts.map((prompt) => ({
              id: prompt.id,
              workspace_id: workspaceId,
              category_id: prompt.categoryId,
              title: prompt.title,
              content: prompt.content,
              favorite: prompt.favorite,
              position: prompt.order,
              created_by: userId,
              updated_by: userId,
              created_at: isoFromSeconds(prompt.createdAt),
              updated_at: isoFromSeconds(prompt.updatedAt),
            })),
          )
          if (promptResponse.error) throw new Error(promptResponse.error.message)

          const tags = personalPrompts.flatMap((prompt) =>
            prompt.tags.map((tag) => ({ prompt_id: prompt.id, tag })),
          )
          if (tags.length > 0) {
            const tagResponse = await auth.client.from('prompt_tags').upsert(tags)
            if (tagResponse.error) throw new Error(tagResponse.error.message)
          }
        }

        if (projects.projects.length > 0) {
          const projectResponse = await auth.client.from('projects').upsert(
            projects.projects.map((project) => ({
              id: project.id,
              workspace_id: workspaceId,
              code: project.code,
              version: project.version,
              name: project.name,
              file_display_ref: project.filePath,
              release_date: project.releaseDate,
              main_stage_key: project.mainStageKey,
              archived: project.archived,
              owner_user_id: project.ownerUserId || userId,
              is_public: project.isPublic,
              last_activity_summary: project.lastActivitySummary,
              last_activity_actor_name: project.lastActivityActorName,
              created_by: userId,
              updated_by: userId,
              created_at: project.createdAt,
              updated_at: project.updatedAt,
            })),
          )
          if (projectResponse.error) throw new Error(projectResponse.error.message)

          const stageResponse = await auth.client.from('project_stages').upsert(
            projects.projects.flatMap((project) =>
              project.stages.map((stage) => ({
                id: stage.id,
                workspace_id: workspaceId,
                project_id: project.id,
                stage_key: stage.stageKey,
                position: stage.position,
                start_date: stage.startDate,
                end_date: stage.endDate,
                progress: stage.progress,
                created_by: userId,
                updated_by: userId,
                updated_at: stage.updatedAt,
              })),
            ),
          )
          if (stageResponse.error) throw new Error(stageResponse.error.message)
        }

        if (daily.day) {
          const dayResponse = await auth.client.from('daily_task_days').upsert({
            id: daily.day.id,
            workspace_id: workspaceId,
            local_date: daily.day.localDate,
            settled_at: daily.day.settledAt,
            report_snapshot: daily.day.reportSnapshot,
            created_by: userId,
            updated_by: userId,
          })
          if (dayResponse.error) throw new Error(dayResponse.error.message)

          if (daily.day.groups.length > 0) {
            const groupResponse = await auth.client.from('daily_task_groups').upsert(
              daily.day.groups.map((group) => ({
                id: group.id,
                workspace_id: workspaceId,
                day_id: daily.day?.id,
                code: group.code,
                project_id: group.projectId,
                position: group.position,
                created_by: userId,
                updated_by: userId,
              })),
            )
            if (groupResponse.error) throw new Error(groupResponse.error.message)
          }

          const tasks = daily.day.groups.flatMap((group) =>
            group.tasks.map((task) => ({
              id: task.id,
              workspace_id: workspaceId,
              group_id: group.id,
              title: task.title,
              progress: task.progress,
              note: task.note,
              invested_minutes: task.investedMinutes,
              reminder_time: task.reminderTime,
              reminder_content: task.reminderContent,
              position: task.position,
              source_task_id: task.sourceTaskId,
              source_snapshot_hash: task.sourceSnapshotHash,
              created_by: userId,
              updated_by: userId,
              created_at: task.createdAt,
              updated_at: task.updatedAt,
            })),
          )
          if (tasks.length > 0) {
            const taskResponse = await auth.client.from('daily_tasks').upsert(tasks)
            if (taskResponse.error) throw new Error(taskResponse.error.message)
          }
        }

        this.status = 'completed'
        this.dismissed = true
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        this.status = 'error'
      }
    },
  },
})

import { defineStore } from 'pinia'
import type { CreateDailyTaskInput, DailyTaskDay, UpdateDailyTaskInput } from '@/domain/production'
import {
  createDailyTask,
  deleteDailyTask,
  loadDailyTaskDay,
  reorderDailyGroups,
  reorderDailyTasks,
  updateDailyTask,
} from '@/lib/productionIpc'

function localToday() {
  const now = new Date()
  const offset = now.getTimezoneOffset() * 60_000
  return new Date(now.getTime() - offset).toISOString().slice(0, 10)
}

export const useDailyTasksStore = defineStore('dailyTasks', {
  state: () => ({
    selectedDate: localToday(),
    day: null as DailyTaskDay | null,
    loading: false,
    error: '',
  }),
  actions: {
    async selectDate(localDate: string) {
      this.selectedDate = localDate
      this.loading = true
      this.error = ''
      try { this.day = await loadDailyTaskDay(localDate) }
      catch (error) { this.error = error instanceof Error ? error.message : String(error) }
      finally { this.loading = false }
    },
    async create(input: Omit<CreateDailyTaskInput, 'localDate' | 'projectId'> & { projectId?: string | null }) {
      await this.replace(() => createDailyTask({ ...input, localDate: this.selectedDate, projectId: input.projectId ?? null }))
    },
    async update(input: UpdateDailyTaskInput) { await this.replace(() => updateDailyTask(input)) },
    async remove(taskId: string) { await this.replace(() => deleteDailyTask(this.selectedDate, taskId)) },
    async reorderGroups(groupIds: string[]) { await this.replace(() => reorderDailyGroups(this.selectedDate, groupIds)) },
    async reorderTasks(groupId: string, taskIds: string[]) { await this.replace(() => reorderDailyTasks(this.selectedDate, groupId, taskIds)) },
    async replace(action: () => Promise<DailyTaskDay>) {
      this.error = ''
      try { this.day = await action() }
      catch (error) { this.error = error instanceof Error ? error.message : String(error); throw error }
    },
  },
})

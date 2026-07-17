import { defineStore } from 'pinia'
import type { SupabaseClient } from '@supabase/supabase-js'
import type { StageKey } from '@/domain/production'

type ScheduleRequestClient = Pick<SupabaseClient, 'from'>
type ScheduleRequestStatus = 'pending' | 'approved' | 'rejected'

export interface ProjectScheduleChangeRequest {
  id: string
  workspaceId: string
  projectId: string
  stageKey: StageKey
  requestedStartDate: string
  requestedEndDate: string
  reason: string
  status: ScheduleRequestStatus
  requestedBy: string
  decidedBy: string | null
  decisionNote: string
  decidedAt: string | null
  createdAt: string
  updatedAt: string
}

interface ScheduleRequestRow {
  id: string
  workspace_id: string
  project_id: string
  stage_key: StageKey
  requested_start_date: string
  requested_end_date: string
  reason: string
  status: ScheduleRequestStatus
  requested_by: string
  decided_by: string | null
  decision_note: string | null
  decided_at: string | null
  created_at: string
  updated_at: string
}

interface CreateScheduleRequestInput {
  workspaceId: string
  projectId: string
  projectOwnerUserId: string
  requesterUserId: string
  stageKey: StageKey
  requestedStartDate: string
  requestedEndDate: string
  reason: string
}

interface DecideScheduleRequestInput {
  requestId: string
  notificationId?: string
  actorUserId: string
  actorName: string
  decisionNote: string
}

function mapRequest(row: ScheduleRequestRow): ProjectScheduleChangeRequest {
  return {
    id: row.id,
    workspaceId: row.workspace_id,
    projectId: row.project_id,
    stageKey: row.stage_key,
    requestedStartDate: row.requested_start_date,
    requestedEndDate: row.requested_end_date,
    reason: row.reason,
    status: row.status,
    requestedBy: row.requested_by,
    decidedBy: row.decided_by,
    decisionNote: row.decision_note ?? '',
    decidedAt: row.decided_at,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

function requestSummary(request: ProjectScheduleChangeRequest, decisionNote: string) {
  const note = decisionNote.trim()
  return `同意调整 ${request.stageKey}：${request.requestedStartDate} 至 ${request.requestedEndDate}${note ? `（${note}）` : ''}`
}

export const useScheduleRequestsStore = defineStore('scheduleRequests', {
  state: () => ({
    loading: false,
    error: '',
    activeRequest: null as ProjectScheduleChangeRequest | null,
  }),
  actions: {
    async createRequest(client: ScheduleRequestClient, input: CreateScheduleRequestInput) {
      const reason = input.reason.trim()
      if (!reason) throw new Error('请填写申请理由')
      if (input.requestedStartDate > input.requestedEndDate) throw new Error('开始时间不能晚于结束时间')

      const response = await client
        .from('project_schedule_change_requests')
        .insert({
          workspace_id: input.workspaceId,
          project_id: input.projectId,
          stage_key: input.stageKey,
          requested_start_date: input.requestedStartDate,
          requested_end_date: input.requestedEndDate,
          reason,
          status: 'pending',
          requested_by: input.requesterUserId,
          created_by: input.requesterUserId,
          updated_by: input.requesterUserId,
        })
        .select('id')
        .single()
      if (response.error) throw new Error(response.error.message)

      const requestId = String(response.data.id)
      const notificationResponse = await client.from('notifications').insert({
        workspace_id: input.workspaceId,
        recipient_user_id: input.projectOwnerUserId,
        actor_user_id: input.requesterUserId,
        kind: 'project_update',
        target_type: 'project_schedule_request',
        target_id: requestId,
        created_by: input.requesterUserId,
        updated_by: input.requesterUserId,
      })
      if (notificationResponse.error) throw new Error(notificationResponse.error.message)

      return { id: requestId }
    },

    async loadRequest(client: ScheduleRequestClient, requestId: string) {
      this.loading = true
      this.error = ''
      try {
        const response = await client
          .from('project_schedule_change_requests')
          .select('*')
          .eq('id', requestId)
          .single()
        if (response.error) throw new Error(response.error.message)
        this.activeRequest = mapRequest(response.data as ScheduleRequestRow)
        return this.activeRequest
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
        throw error
      } finally {
        this.loading = false
      }
    },

    async approveRequest(client: ScheduleRequestClient, input: DecideScheduleRequestInput) {
      const request = await this.loadRequest(client, input.requestId)
      const now = new Date().toISOString()
      const summary = requestSummary(request, input.decisionNote)

      const decisionResponse = await client
        .from('project_schedule_change_requests')
        .update({
          status: 'approved',
          decided_by: input.actorUserId,
          decision_note: input.decisionNote.trim(),
          decided_at: now,
          updated_by: input.actorUserId,
          updated_at: now,
        })
        .eq('id', input.requestId)
      if (decisionResponse.error) throw new Error(decisionResponse.error.message)

      const stageResponse = await client
        .from('project_stages')
        .update({
          start_date: request.requestedStartDate,
          end_date: request.requestedEndDate,
          updated_by: input.actorUserId,
          updated_at: now,
        })
        .eq('project_id', request.projectId)
        .eq('stage_key', request.stageKey)
      if (stageResponse.error) throw new Error(stageResponse.error.message)

      const projectResponse = await client
        .from('projects')
        .update({
          last_activity_summary: summary,
          last_activity_actor_name: input.actorName,
          updated_by: input.actorUserId,
          updated_at: now,
        })
        .eq('id', request.projectId)
      if (projectResponse.error) throw new Error(projectResponse.error.message)

      const activityResponse = await client.from('project_activity_log').insert({
        workspace_id: request.workspaceId,
        project_id: request.projectId,
        actor_user_id: input.actorUserId,
        actor_name: input.actorName,
        summary,
      })
      if (activityResponse.error) throw new Error(activityResponse.error.message)

      if (input.notificationId) await this.markNotificationRead(client, input.notificationId, input.actorUserId, now)

      this.activeRequest = { ...request, status: 'approved', decidedBy: input.actorUserId, decisionNote: input.decisionNote, decidedAt: now }
      return this.activeRequest
    },

    async rejectRequest(client: ScheduleRequestClient, input: DecideScheduleRequestInput) {
      const request = await this.loadRequest(client, input.requestId)
      const now = new Date().toISOString()
      const note = input.decisionNote.trim()
      const summary = `拒绝调整 ${request.stageKey}${note ? `：${note}` : ''}`

      const decisionResponse = await client
        .from('project_schedule_change_requests')
        .update({
          status: 'rejected',
          decided_by: input.actorUserId,
          decision_note: note,
          decided_at: now,
          updated_by: input.actorUserId,
          updated_at: now,
        })
        .eq('id', input.requestId)
      if (decisionResponse.error) throw new Error(decisionResponse.error.message)

      const activityResponse = await client.from('project_activity_log').insert({
        workspace_id: request.workspaceId,
        project_id: request.projectId,
        actor_user_id: input.actorUserId,
        actor_name: input.actorName,
        summary,
      })
      if (activityResponse.error) throw new Error(activityResponse.error.message)

      if (input.notificationId) await this.markNotificationRead(client, input.notificationId, input.actorUserId, now)

      this.activeRequest = { ...request, status: 'rejected', decidedBy: input.actorUserId, decisionNote: note, decidedAt: now }
      return this.activeRequest
    },

    async markNotificationRead(client: ScheduleRequestClient, notificationId: string, actorUserId: string, now: string) {
      const response = await client
        .from('notifications')
        .update({ read_at: now, updated_at: now, updated_by: actorUserId })
        .eq('id', notificationId)
      if (response.error) throw new Error(response.error.message)
    },
  },
})

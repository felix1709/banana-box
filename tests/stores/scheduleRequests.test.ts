import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useScheduleRequestsStore } from '@/stores/scheduleRequests'

describe('schedule requests store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('creates a project schedule change request and notifies the project owner', async () => {
    const store = useScheduleRequestsStore()
    const client = createClientMock({
      project_schedule_change_requests: {
        insertResult: { data: { id: 'request-1' }, error: null },
      },
      notifications: {
        insertResult: { data: [], error: null },
      },
    })

    const request = await store.createRequest(client as never, {
      workspaceId: 'workspace-1',
      projectId: 'project-1',
      projectOwnerUserId: 'owner-1',
      requesterUserId: 'user-2',
      stageKey: 'storyboard',
      requestedStartDate: '2026-07-03',
      requestedEndDate: '2026-07-12',
      reason: '分镜素材比预期晚两天到齐',
    })

    expect(request.id).toBe('request-1')
    expect(client.tables.project_schedule_change_requests.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      project_id: 'project-1',
      stage_key: 'storyboard',
      requested_start_date: '2026-07-03',
      requested_end_date: '2026-07-12',
      reason: '分镜素材比预期晚两天到齐',
      status: 'pending',
      requested_by: 'user-2',
      created_by: 'user-2',
      updated_by: 'user-2',
    }))
    expect(client.tables.notifications.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      recipient_user_id: 'owner-1',
      actor_user_id: 'user-2',
      kind: 'project_update',
      target_type: 'project_schedule_request',
      target_id: 'request-1',
    }))
  })

  it('approves a schedule request, updates the stage, writes activity, and marks the notification read', async () => {
    const store = useScheduleRequestsStore()
    const client = createClientMock({
      project_schedule_change_requests: {
        rows: [{
          id: 'request-1',
          workspace_id: 'workspace-1',
          project_id: 'project-1',
          stage_key: 'storyboard',
          requested_start_date: '2026-07-03',
          requested_end_date: '2026-07-12',
          reason: '分镜素材比预期晚两天到齐',
          status: 'pending',
          requested_by: 'user-2',
          decided_by: null,
          decision_note: '',
          decided_at: null,
          created_at: '2026-07-17T01:00:00Z',
          updated_at: '2026-07-17T01:00:00Z',
        }],
        updateResult: { data: [], error: null },
      },
      project_stages: {
        updateResult: { data: [], error: null },
      },
      projects: {
        updateResult: { data: [], error: null },
      },
      project_activity_log: {
        insertResult: { data: [], error: null },
      },
      notifications: {
        updateResult: { data: [], error: null },
      },
    })

    const result = await store.approveRequest(client as never, {
      requestId: 'request-1',
      notificationId: 'notification-1',
      actorUserId: 'owner-1',
      actorName: '导演',
      decisionNote: '同意调整',
    })

    expect(result.status).toBe('approved')
    expect(client.tables.project_stages.update).toHaveBeenCalledWith(expect.objectContaining({
      start_date: '2026-07-03',
      end_date: '2026-07-12',
      updated_by: 'owner-1',
    }))
    expect(client.tables.project_activity_log.insert).toHaveBeenCalledWith(expect.objectContaining({
      workspace_id: 'workspace-1',
      project_id: 'project-1',
      actor_user_id: 'owner-1',
      actor_name: '导演',
      summary: expect.stringContaining('同意调整'),
    }))
    expect(client.tables.notifications.update).toHaveBeenCalledWith(expect.objectContaining({
      read_at: expect.any(String),
      updated_by: 'owner-1',
    }))
  })
})

function createClientMock(tableSetup: Record<string, {
  rows?: unknown[]
  insertResult?: { data: unknown, error: null | { message: string } }
  updateResult?: { data: unknown, error: null | { message: string } }
}>) {
  const tables: Record<string, {
    insert: ReturnType<typeof vi.fn>
    update: ReturnType<typeof vi.fn>
    select: ReturnType<typeof vi.fn>
  }> = {}

  return {
    tables,
    from: vi.fn((table: string) => {
      const setup = tableSetup[table] ?? {}
      const tableMock = {
        insert: vi.fn(() => ({
          select: vi.fn(() => ({
            single: vi.fn(async () => setup.insertResult ?? { data: {}, error: null }),
          })),
        })),
        update: vi.fn(() => updateBuilder(setup.updateResult ?? { data: [], error: null })),
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            single: vi.fn(async () => ({ data: setup.rows?.[0] ?? null, error: null })),
          })),
        })),
      }
      tables[table] = tableMock
      return tableMock
    }),
  }
}

function updateBuilder(result: { data: unknown, error: null | { message: string } }) {
  const builder = {
    eq: vi.fn(() => builder),
    then: (resolve: (value: typeof result) => unknown, reject: (reason: unknown) => unknown) =>
      Promise.resolve(result).then(resolve, reject),
  }
  return builder
}

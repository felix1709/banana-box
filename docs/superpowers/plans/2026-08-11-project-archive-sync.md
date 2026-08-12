# Project Archive Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复项目归档后点击同步又出现在进行中列表的问题，并让归档/恢复在有云端工作区时写回云端。

**Architecture:** 保持现有 Pinia store 结构。`syncStatus` 负责拉取云端表，`projects` store 负责项目集合合并；合并时增加更新时间判断，避免旧云端项目覆盖本地新状态。项目页在归档/恢复后调用已有 `ensureCloudProject()` 上传更新后的项目状态。

**Tech Stack:** Vue 3, Pinia, Vitest, Supabase client mock, Tauri IPC mock.

## Global Constraints

- 默认中文用户文案。
- 不重写完整 outbox/revision 同步引擎。
- 不触碰历史 Rust 脏文件。
- 先写失败测试，再写实现。

---

### Task 1: Protect local archive state during cloud pull

**Files:**
- Modify: `src/stores/projects.ts`
- Test: `tests/stores/syncStatus.test.ts`

**Interfaces:**
- Consumes: `useProjectsStore().mergeProjects(projects: Project[])`
- Produces: `mergeProjects()` 保留更新时间更新的本地项目。

- [ ] **Step 1: Write the failing test**

```ts
it('keeps a newer local archived project when cloud pull returns an older active copy', async () => {
  const store = useSyncStatusStore()
  const projects = useProjectsStore()
  projects.hydrate([projectFixture({
    id: 'project-1',
    archived: true,
    updatedAt: '2026-08-11T10:00:00Z',
  })])
  const client = tableClientMock({
    projects: [cloudProjectRow({
      id: 'project-1',
      archived: false,
      updated_at: '2026-08-10T10:00:00Z',
    })],
  })

  await store.pullWorkspace(client as never, 'workspace-1')

  expect(projects.projects.find((project) => project.id === 'project-1')?.archived).toBe(true)
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run tests/stores/syncStatus.test.ts`

Expected: FAIL because current `mergeProjects()` overwrites local with cloud.

- [ ] **Step 3: Write minimal implementation**

Add a helper in `src/stores/projects.ts`:

```ts
function projectTimestamp(project: Project) {
  const timestamp = Date.parse(project.updatedAt)
  return Number.isFinite(timestamp) ? timestamp : 0
}
```

Then update `mergeProjects()` so an incoming project replaces the existing one only when the incoming timestamp is newer or equal.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run tests/stores/syncStatus.test.ts`

Expected: PASS.

### Task 2: Verify newer cloud state can still restore projects

**Files:**
- Test: `tests/stores/syncStatus.test.ts`

**Interfaces:**
- Consumes: updated `mergeProjects()`
- Produces: confidence that the fix does not block legitimate newer cloud updates.

- [ ] **Step 1: Write test**

```ts
it('accepts a newer cloud restore state over an older local archive state', async () => {
  const store = useSyncStatusStore()
  const projects = useProjectsStore()
  projects.hydrate([projectFixture({
    id: 'project-1',
    archived: true,
    updatedAt: '2026-08-10T10:00:00Z',
  })])
  const client = tableClientMock({
    projects: [cloudProjectRow({
      id: 'project-1',
      archived: false,
      updated_at: '2026-08-11T10:00:00Z',
    })],
  })

  await store.pullWorkspace(client as never, 'workspace-1')

  expect(projects.projects.find((project) => project.id === 'project-1')?.archived).toBe(false)
})
```

- [ ] **Step 2: Run test**

Run: `pnpm vitest run tests/stores/syncStatus.test.ts`

Expected: PASS.

### Task 3: Upload archive and restore changes when cloud is available

**Files:**
- Modify: `src/components/projects/ProjectBoardPage.vue`
- Test: `tests/components/ProjectBoardPage.test.ts`

**Interfaces:**
- Consumes: `projects.setArchived(projectId, archived)` and `projects.ensureCloudProject(client, workspaceId, userId, projectId)`
- Produces: project card archive/restore actions write the updated project to cloud when auth and workspace are available.

- [ ] **Step 1: Write failing component test**

Mock `archiveProject()` to return the updated project, click archive, and assert the cloud client receives an upsert to `projects` with `archived: true`.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run tests/components/ProjectBoardPage.test.ts`

Expected: FAIL because archive/restore currently only call local `setArchived()`.

- [ ] **Step 3: Implement minimal upload helper**

In `ProjectBoardPage.vue`, after local archive/restore succeeds, call:

```ts
await syncProjectArchiveState(projectId)
```

The helper should no-op unless `auth.client`, `auth.user`, and `workspaces.activeWorkspaceId` exist.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run tests/components/ProjectBoardPage.test.ts`

Expected: PASS.

### Task 4: Final verification

**Files:**
- No source edits unless a verification failure identifies a specific bug.

- [ ] **Step 1: Run targeted project tests**

Run:

```powershell
pnpm vitest run tests/stores/projects.test.ts tests/stores/syncStatus.test.ts tests/components/ProjectBoardPage.test.ts tests/stores/cloudMigration.test.ts tests/components/SyncStatusIndicator.test.ts tests/components/WorkspaceSwitcher.test.ts
```

Expected: all tests pass.

- [ ] **Step 2: Run broader frontend check**

Run:

```powershell
pnpm check
```

Expected: all frontend checks pass.
